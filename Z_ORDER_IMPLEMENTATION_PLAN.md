# WRY Z-Order API Implementation Plan

## Executive Summary

This document outlines the implementation plan for adding dynamic z-order control to WRY webviews. The goal is to allow webviews to be reordered at runtime, enabling UI overlays to appear above content webviews when needed.

**Primary target**: macOS implementation for the eidolon browser project, with stubs for other platforms.

---

## Problem Statement

Currently, WRY webviews are z-ordered by creation order - later-created webviews appear on top. This order cannot be changed at runtime, which causes problems for applications like eidolon where UI elements (modals, dropdowns) need to render above content webviews.

### Related Issues
- **Tauri #9798**: New webview doesn't appear on top on Windows
- **Tauri #12450**: Transparent child window issues
- **WRY Discussion #458**: Multiple webviews in single window

---

## Full Integration Path: WRY → Tauri

Understanding the full stack is critical for a proper implementation:

```
┌─────────────────────────────────────────────────────────────────┐
│  Tauri App (JavaScript/TypeScript)                              │
│  └─ @tauri-apps/api/webview                                     │
│       └─ webview.bringToFront()  [NEW - invoke command]         │
├─────────────────────────────────────────────────────────────────┤
│  Tauri Core (Rust)                                              │
│  └─ tauri/src/webview/mod.rs                                    │
│       └─ Webview::bring_to_front()  [NEW - delegates to runtime]│
├─────────────────────────────────────────────────────────────────┤
│  Tauri Runtime (trait)                                          │
│  └─ tauri-runtime/src/lib.rs                                    │
│       └─ WebviewDispatch::bring_to_front()  [NEW - trait method]│
├─────────────────────────────────────────────────────────────────┤
│  Tauri Runtime WRY (implementation)                             │
│  └─ tauri-runtime-wry/src/lib.rs                                │
│       └─ impl WebviewDispatch for WryWebviewDispatcher          │
│            └─ bring_to_front() { webview.bring_to_front() }     │
├─────────────────────────────────────────────────────────────────┤
│  WRY (this crate)                                               │
│  └─ src/lib.rs                                                  │
│       └─ WebView::bring_to_front()  [NEW - public API]          │
│       └─ WebViewExtMacOS::bring_to_front()  [NEW - trait]       │
│  └─ src/wkwebview/mod.rs                                        │
│       └─ InnerWebView::bring_to_front()  [NEW - implementation] │
└─────────────────────────────────────────────────────────────────┘
```

### Files Changed Per Layer

| Layer | Crate | Files to Modify |
|-------|-------|-----------------|
| **WRY** | `wry` | `src/lib.rs`, `src/wkwebview/mod.rs`, `src/webview2/mod.rs`, `src/error.rs` |
| **Runtime Trait** | `tauri-runtime` | `src/lib.rs` (WebviewDispatch trait) |
| **Runtime Impl** | `tauri-runtime-wry` | `src/lib.rs` (WryWebviewDispatcher impl) |
| **Tauri Core** | `tauri` | `src/webview/mod.rs` (Webview struct) |
| **JS API** | `@tauri-apps/api` | `src/webview.ts` |

---

## Proposed WRY API

### Option A: Cross-Platform Methods (Recommended)

Add methods directly to `WebView` that work on all platforms:

```rust
impl WebView {
    /// Moves this webview to the front (top) of the z-order within its parent.
    /// After calling this, the webview will render on top of all sibling webviews.
    ///
    /// ## Platform-specific
    ///
    /// - **macOS**: Uses NSView z-ordering
    /// - **Windows**: Uses SetWindowPos with HWND_TOP
    /// - **Linux**: Not yet implemented, returns Ok(()) as no-op
    /// - **Android/iOS**: Not supported
    pub fn bring_to_front(&self) -> Result<()>;

    /// Moves this webview to the back (bottom) of the z-order within its parent.
    /// After calling this, the webview will render behind all sibling webviews.
    ///
    /// ## Platform-specific
    ///
    /// - **macOS**: Uses NSView z-ordering
    /// - **Windows**: Uses SetWindowPos with HWND_BOTTOM
    /// - **Linux**: Not yet implemented, returns Ok(()) as no-op
    /// - **Android/iOS**: Not supported
    pub fn send_to_back(&self) -> Result<()>;
}
```

**Rationale**: This matches how other WRY methods work (e.g., `set_visible`, `set_bounds`, `focus`). It allows Tauri to expose a single cross-platform API.

### Option B: Platform-Specific Extensions Only

Only add to platform extension traits (`WebViewExtMacOS`, `WebViewExtWindows`).

**Rationale**: More explicit about platform support, but requires Tauri to handle platform dispatch.

### Recommendation: **Option A** (Cross-Platform)

This is simpler for Tauri integration and follows WRY's existing patterns. The implementation can be platform-specific internally while exposing a unified API.

---

## macOS Implementation (Primary Focus)

### Platform API

**`NSView.addSubview(_:positioned:relativeTo:)`**

```objc
- (void)addSubview:(NSView *)view
        positioned:(NSWindowOrderingMode)place
        relativeTo:(NSView *)otherView;
```

- `place`: `.above` or `.below`
- `otherView`: Reference view, or `nil` for extreme position
- When `otherView` is `nil`:
  - `.above` → top of z-order (frontmost)
  - `.below` → bottom of z-order (backmost)

### Implementation in `src/wkwebview/mod.rs`

```rust
impl InnerWebView {
    /// Brings this webview to the front of the z-order
    #[cfg(target_os = "macos")]
    pub fn bring_to_front(&self) -> crate::Result<()> {
        use objc2_app_kit::NSWindowOrderingMode;

        unsafe {
            // The webview might be inside WryWebViewParent or directly in ns_view
            // We need to reorder within the immediate superview
            if let Some(superview) = self.webview.superview() {
                superview.addSubview_positioned_relativeTo(
                    &self.webview,
                    NSWindowOrderingMode::Above,
                    None, // nil = frontmost position
                );
            }
        }
        Ok(())
    }

    /// Sends this webview to the back of the z-order
    #[cfg(target_os = "macos")]
    pub fn send_to_back(&self) -> crate::Result<()> {
        use objc2_app_kit::NSWindowOrderingMode;

        unsafe {
            if let Some(superview) = self.webview.superview() {
                superview.addSubview_positioned_relativeTo(
                    &self.webview,
                    NSWindowOrderingMode::Below,
                    None, // nil = backmost position
                );
            }
        }
        Ok(())
    }
}
```

### Key Considerations for macOS

1. **Parent View Structure**:
   - Non-child webviews: `WryWebViewParent` → `WryWebView`
   - Child webviews: `ns_view` → `WryWebView` (via `addSubview`)
   - Z-ordering operates on siblings within the same parent

2. **WryWebViewParent** (`src/wkwebview/class/wry_web_view_parent.rs`):
   - Custom NSView subclass wrapping the webview
   - Only used for non-child (main) webviews
   - Handles traffic light positioning and key events

3. **Thread Safety**:
   - `MainThreadMarker` ensures we're on the main thread
   - NSView operations must be on main thread (already enforced by WRY)

---

## Other Platform Stubs

### Windows (`src/webview2/mod.rs`)

```rust
impl InnerWebView {
    /// Brings this webview to the front of the z-order
    ///
    /// TODO: Implement using SetWindowPos with HWND_TOP
    /// See: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowpos
    pub fn bring_to_front(&self) -> crate::Result<()> {
        // Stub: Windows implementation pending
        // Will use: SetWindowPos(self.hwnd, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE)
        Ok(())
    }

    /// Sends this webview to the back of the z-order
    ///
    /// TODO: Implement using SetWindowPos with HWND_BOTTOM
    pub fn send_to_back(&self) -> crate::Result<()> {
        // Stub: Windows implementation pending
        // Will use: SetWindowPos(self.hwnd, HWND_BOTTOM, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE)
        Ok(())
    }
}
```

### Linux (`src/webkitgtk/mod.rs`)

```rust
impl InnerWebView {
    /// Brings this webview to the front of the z-order
    ///
    /// TODO: Research GTK z-ordering options:
    /// - For gtk::Fixed: remove and re-add widget (later additions on top)
    /// - For gtk::Box: may need to use gtk::Overlay instead
    /// - CSS z-index only works in GTK4, WRY uses GTK3
    pub fn bring_to_front(&self) -> crate::Result<()> {
        // Stub: Linux implementation requires GTK research
        // Current approach: no-op, return success
        Ok(())
    }

    /// Sends this webview to the back of the z-order
    ///
    /// TODO: Research GTK z-ordering options
    pub fn send_to_back(&self) -> crate::Result<()> {
        // Stub: Linux implementation requires GTK research
        Ok(())
    }
}
```

### Android (`src/android/mod.rs`)

```rust
impl InnerWebView {
    /// Z-order control is not supported on Android
    pub fn bring_to_front(&self) -> crate::Result<()> {
        // Android WebView doesn't support programmatic z-ordering
        // within the view hierarchy in the same way
        Ok(())
    }

    pub fn send_to_back(&self) -> crate::Result<()> {
        Ok(())
    }
}
```

---

## Public API Changes in `src/lib.rs`

### Add to `WebView` struct

```rust
impl WebView {
    // ... existing methods ...

    /// Moves this webview to the front (top) of the z-order within its parent.
    ///
    /// After calling this, the webview will render on top of all sibling webviews.
    /// This is useful for bringing UI overlay webviews above content webviews.
    ///
    /// ## Platform-specific
    ///
    /// - **macOS**: Implemented using NSView z-ordering
    /// - **Windows**: Stub (returns Ok), implementation pending
    /// - **Linux**: Stub (returns Ok), implementation pending
    /// - **Android/iOS**: No-op (returns Ok)
    ///
    /// ## Example
    ///
    /// ```no_run
    /// # use wry::WebViewBuilder;
    /// # let window = todo!();
    /// let content_webview = WebViewBuilder::new()
    ///     .with_url("https://example.com")
    ///     .build_as_child(&window)?;
    ///
    /// let ui_webview = WebViewBuilder::new()
    ///     .with_html("<h1>UI Overlay</h1>")
    ///     .build_as_child(&window)?;
    ///
    /// // Later, bring content to front
    /// content_webview.bring_to_front()?;
    /// # Ok::<(), wry::Error>(())
    /// ```
    pub fn bring_to_front(&self) -> Result<()> {
        self.webview.bring_to_front()
    }

    /// Moves this webview to the back (bottom) of the z-order within its parent.
    ///
    /// After calling this, the webview will render behind all sibling webviews.
    ///
    /// ## Platform-specific
    ///
    /// - **macOS**: Implemented using NSView z-ordering
    /// - **Windows**: Stub (returns Ok), implementation pending
    /// - **Linux**: Stub (returns Ok), implementation pending
    /// - **Android/iOS**: No-op (returns Ok)
    pub fn send_to_back(&self) -> Result<()> {
        self.webview.send_to_back()
    }
}
```

### Optionally extend platform traits

For more advanced use cases (relative positioning), we can add to platform extension traits:

```rust
#[cfg(target_os = "macos")]
pub trait WebViewExtMacOS {
    // ... existing methods ...

    /// Positions this webview directly above another webview in the z-order.
    ///
    /// Returns an error if the webviews don't share the same parent view.
    fn position_above(&self, other: &WebView) -> Result<()>;

    /// Positions this webview directly below another webview in the z-order.
    ///
    /// Returns an error if the webviews don't share the same parent view.
    fn position_below(&self, other: &WebView) -> Result<()>;
}
```

---

## Tauri Integration (Downstream)

Once WRY has the API, Tauri needs these changes:

### 1. `tauri-runtime` - Add to WebviewDispatch trait

```rust
// In tauri-runtime/src/lib.rs
pub trait WebviewDispatch<T: UserEvent>: ... {
    // ... existing methods ...

    /// Brings the webview to the front of the z-order
    fn bring_to_front(&self) -> Result<()>;

    /// Sends the webview to the back of the z-order
    fn send_to_back(&self) -> Result<()>;
}
```

### 2. `tauri-runtime-wry` - Implement the trait

```rust
// In tauri-runtime-wry/src/lib.rs
impl<T: UserEvent> WebviewDispatch<T> for WryWebviewDispatcher {
    fn bring_to_front(&self) -> Result<()> {
        self.webview.bring_to_front().map_err(|e| /* convert error */)
    }

    fn send_to_back(&self) -> Result<()> {
        self.webview.send_to_back().map_err(|e| /* convert error */)
    }
}
```

### 3. `tauri` - Expose on Webview struct

```rust
// In tauri/src/webview/mod.rs
impl<R: Runtime> Webview<R> {
    /// Brings this webview to the front of the z-order
    pub fn bring_to_front(&self) -> crate::Result<()> {
        self.webview.dispatcher.bring_to_front().map_err(Into::into)
    }

    /// Sends this webview to the back of the z-order
    pub fn send_to_back(&self) -> crate::Result<()> {
        self.webview.dispatcher.send_to_back().map_err(Into::into)
    }
}
```

### 4. JavaScript API (optional)

```typescript
// In @tauri-apps/api/webview.ts
class Webview {
    /** Brings this webview to the front of the z-order */
    async bringToFront(): Promise<void> {
        return invoke('plugin:webview|bring_to_front', { label: this.label });
    }

    /** Sends this webview to the back of the z-order */
    async sendToBack(): Promise<void> {
        return invoke('plugin:webview|send_to_back', { label: this.label });
    }
}
```

---

## Test Strategy

### WRY Example: `examples/z_order_test.rs`

```rust
//! Z-Order Test Example
//!
//! Creates two overlapping webviews and allows toggling which is on top.
//! Press 'T' to toggle, 'F' for content front, 'B' for content back, 'Q' to quit.

use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::KeyCode,
    window::WindowBuilder,
};
use wry::{dpi::LogicalPosition, dpi::LogicalSize, Rect, WebViewBuilder};

fn main() -> wry::Result<()> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Z-Order Test - Press T to toggle, Q to quit")
        .with_inner_size(tao::dpi::LogicalSize::new(600, 450))
        .build(&event_loop)
        .unwrap();

    // Create content webview (red, created first = initially behind)
    let content_wv = WebViewBuilder::new()
        .with_html(r#"
            <!DOCTYPE html>
            <html>
            <body style="background: #e74c3c; margin: 0; display: flex;
                         align-items: center; justify-content: center; height: 100vh;">
                <h1 style="color: white; font-family: system-ui;">Content WebView (Red)</h1>
            </body>
            </html>
        "#)
        .with_bounds(Rect {
            position: LogicalPosition::new(50, 50).into(),
            size: LogicalSize::new(350, 250).into(),
        })
        .build_as_child(&window)?;

    // Create UI webview (blue, created second = initially on top)
    let ui_wv = WebViewBuilder::new()
        .with_html(r#"
            <!DOCTYPE html>
            <html>
            <body style="background: #3498db; margin: 0; display: flex;
                         align-items: center; justify-content: center; height: 100vh;">
                <h1 style="color: white; font-family: system-ui;">UI WebView (Blue)</h1>
            </body>
            </html>
        "#)
        .with_bounds(Rect {
            position: LogicalPosition::new(200, 150).into(),
            size: LogicalSize::new(350, 250).into(),
        })
        .build_as_child(&window)?;

    let mut content_on_top = false;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        if let Event::WindowEvent { event, .. } = event {
            match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::KeyboardInput { event, .. } => {
                    if event.state == tao::event::ElementState::Pressed {
                        match event.physical_key {
                            tao::keyboard::PhysicalKey::Code(KeyCode::KeyT) => {
                                // Toggle
                                if content_on_top {
                                    let _ = ui_wv.bring_to_front();
                                    println!("UI webview (blue) brought to front");
                                } else {
                                    let _ = content_wv.bring_to_front();
                                    println!("Content webview (red) brought to front");
                                }
                                content_on_top = !content_on_top;
                            }
                            tao::keyboard::PhysicalKey::Code(KeyCode::KeyF) => {
                                let _ = content_wv.bring_to_front();
                                content_on_top = true;
                                println!("Content webview (red) brought to front");
                            }
                            tao::keyboard::PhysicalKey::Code(KeyCode::KeyB) => {
                                let _ = content_wv.send_to_back();
                                content_on_top = false;
                                println!("Content webview (red) sent to back");
                            }
                            tao::keyboard::PhysicalKey::Code(KeyCode::KeyQ) => {
                                *control_flow = ControlFlow::Exit;
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    });
}
```

### Manual Test Cases

1. **Basic toggle**: Press T repeatedly, verify webviews swap
2. **Multiple webviews**: Create 3+ webviews, verify each can be brought to front
3. **Focus preservation**: Verify focus isn't affected by z-order changes
4. **Resize persistence**: Resize window, verify z-order maintained
5. **Visibility interaction**: Hide/show webviews, verify z-order maintained

---

## Implementation Checklist

### Phase 1: WRY macOS Implementation ✓

- [ ] Add `bring_to_front()` to `InnerWebView` in `src/wkwebview/mod.rs`
- [ ] Add `send_to_back()` to `InnerWebView` in `src/wkwebview/mod.rs`
- [ ] Add stubs to `src/webview2/mod.rs` (Windows)
- [ ] Add stubs to `src/webkitgtk/mod.rs` (Linux) if exists
- [ ] Add stubs to `src/android/mod.rs` (Android)
- [ ] Add public API to `WebView` in `src/lib.rs`
- [ ] Create `examples/z_order_test.rs`
- [ ] Test on macOS
- [ ] Update CLAUDE.md with progress

### Phase 2: Tauri Integration (Separate PR to Tauri)

- [ ] Add to `WebviewDispatch` trait in `tauri-runtime`
- [ ] Implement in `tauri-runtime-wry`
- [ ] Expose on `Webview` struct in `tauri`
- [ ] Add JavaScript API bindings
- [ ] Update Tauri documentation

### Phase 3: Windows Implementation (Future)

- [ ] Implement using `SetWindowPos`
- [ ] Test on Windows
- [ ] Address Tauri issue #9798

---

## Open Questions

1. **Should `position_above`/`position_below` be included?**
   - Pro: More flexibility for complex layouts
   - Con: Requires passing webview references, more complex API
   - **Recommendation**: Start with just `bring_to_front`/`send_to_back`, add relative positioning later if needed

2. **Error handling for no-op platforms?**
   - Current plan: Return `Ok(())` for unimplemented platforms
   - Alternative: Return specific error like `Error::NotSupported`
   - **Recommendation**: Return `Ok(())` to allow cross-platform code without platform checks

3. **Should we expose z-index as a numeric value?**
   - `set_z_index(i32)` / `z_index() -> i32`
   - Adds complexity, platforms handle z-order differently
   - **Recommendation**: No, just front/back operations are sufficient

---

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `src/wkwebview/mod.rs` | Modify | Add `bring_to_front`, `send_to_back` implementations |
| `src/webview2/mod.rs` | Modify | Add stub implementations with TODO comments |
| `src/webkitgtk/mod.rs` | Modify | Add stub implementations with TODO comments |
| `src/android/mod.rs` | Modify | Add no-op implementations |
| `src/lib.rs` | Modify | Add public API on `WebView` struct |
| `examples/z_order_test.rs` | Create | Interactive test example |
| `CLAUDE.md` | Modify | Document progress |

---

## References

- [NSView.addSubview:positioned:relativeTo:](https://developer.apple.com/documentation/appkit/nsview/1483351-addsubview)
- [SetWindowPos](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowpos)
- [Tauri WebviewDispatch trait](https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-runtime/src/lib.rs)
- [Tauri Issue #9798 - Z-order bug on Windows](https://github.com/tauri-apps/tauri/issues/9798)
