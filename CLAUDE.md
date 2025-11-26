# CLAUDE.md

This file provides guidance to Claude Code when working with this WRY fork.

## Project Overview

WRY is a cross-platform WebView rendering library for Rust, part of the Tauri ecosystem. It provides a unified API to create webviews using platform-native web engines:
- **macOS/iOS**: WKWebView (WebKit)
- **Windows**: WebView2 (Edge Chromium)
- **Linux**: WebKitGTK
- **Android**: Android WebView

## Build Commands

```bash
cargo build                    # Build the library
cargo build --all-features     # Build with all features
cargo test                     # Run tests
cargo run --example z_order_test  # Run z-order test example
cargo fmt                      # Format code (2-space indentation)
cargo clippy                   # Run linter
```

## Architecture

### Platform Implementations

Each platform has its own `InnerWebView` in `src/`:

- `src/wkwebview/` - macOS and iOS (WKWebView)
- `src/webview2/` - Windows (WebView2/Edge Chromium)
- `src/webkitgtk/` - Linux (WebKitGTK)
- `src/android/` - Android WebView

### Core Types (`src/lib.rs`)

- `WebView` - Main webview handle, wraps platform-specific `InnerWebView`
- `WebViewBuilder` - Builder pattern for creating webviews
- `WebViewAttributes` - Configuration options

## Code Style

- 2-space indentation (see `rustfmt.toml`)
- Max line width: 100 characters
- License header required: Apache-2.0 OR MIT

---

## Fork Purpose: Z-Order Control for Eidolon

This fork adds **dynamic z-order control** for webviews, developed for the **eidolon** browser project - a Tauri app with TypeScript/React frontend that uses multiple webviews per window.

### The Problem

In a multi-webview browser, UI elements (modals, dropdowns) from one webview render UNDER content webviews because z-order is fixed at creation time.

### The Solution: Dynamic Z-Order API

**Implemented and working on macOS:**

```rust
// Move webview to front (top of z-order)
webview.bring_to_front()?;

// Move webview to back (bottom of z-order)
webview.send_to_back()?;
```

### Implementation Status

| Platform | Status | Implementation |
|----------|--------|----------------|
| **macOS** | ✅ Complete | `NSView.addSubview:positioned:relativeTo:` |
| Windows | Stub | TODO: `SetWindowPos` with `HWND_TOP/BOTTOM` |
| Linux | Stub | TODO: GTK widget re-parenting |
| iOS | Stub | TODO: `UIView.bringSubviewToFront` |
| Android | No-op | Not supported |

### Files Changed

- `src/lib.rs` - Public `bring_to_front()` / `send_to_back()` on `WebView`
- `src/wkwebview/mod.rs` - macOS implementation + iOS stub
- `src/webview2/mod.rs` - Windows stub with TODO
- `src/webkitgtk/mod.rs` - Linux stub with TODO
- `src/android/mod.rs` - Android no-op
- `examples/z_order_test.rs` - Interactive test (press T to toggle)

### Test the Implementation

```bash
cargo run --example z_order_test
# Press T to toggle z-order, Q to quit
```

---

## Tauri Integration Path

To use this in eidolon (or any Tauri app), the following Tauri-side changes are needed:

### 1. `tauri-runtime` crate
Add to `WebviewDispatch` trait:
```rust
fn bring_to_front(&self) -> Result<()>;
fn send_to_back(&self) -> Result<()>;
```

### 2. `tauri-runtime-wry` crate
Implement the trait methods, calling WRY's API.

### 3. `tauri` crate
Expose on `Webview` struct:
```rust
impl Webview {
    pub fn bring_to_front(&self) -> Result<()>;
    pub fn send_to_back(&self) -> Result<()>;
}
```

### 4. Frontend (TypeScript/React)
Call via Tauri command:
```typescript
// In a Tauri command handler or via invoke
await invoke('bring_webview_to_front', { label: 'content-1' });
```

See `Z_ORDER_IMPLEMENTATION_PLAN.md` for full details.

---

## Other APIs Added

### Snapshot API (macOS only)

For performance testing of offscreen rendering:

```rust
use wry::WebViewExtMacOS;

webview.take_snapshot(|duration, image_data| {
    println!("Snapshot took {:?}", duration);
    // image_data is Option<Vec<u8>> in TIFF format
});
```

Benchmarks show ~1.4ms average latency - viable for 60fps if needed.

---

## Key Findings from Research

1. **Sibling webviews respect z-order** - Later `addSubview` calls place views on top
2. **Dynamic reordering works** - `addSubview:positioned:relativeTo:` moves existing views
3. **Transparent overlays work** - But have input passthrough issues (top layer captures all events)
4. **Best approach**: Dynamic z-order (this implementation), not transparency tricks

## Related Files

- `Z_ORDER_IMPLEMENTATION_PLAN.md` - Detailed implementation plan
- `examples/z_order_test.rs` - Interactive z-order test
- `examples/snapshot_timing.rs` - Snapshot performance benchmark
- `examples/compositing_test.rs` - Original z-order research test
