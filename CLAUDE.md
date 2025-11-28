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

## Tauri Integration (Complete)

A companion Tauri fork at `/Volumes/Vault/Workspace/tauri` provides full integration:

### Changes Made

1. **`tauri-runtime` crate** (`crates/tauri-runtime/src/lib.rs`)
   - Added `bring_to_front()` and `send_to_back()` to `WebviewDispatch` trait
   - Added `HitTestMode` enum and `HitRegionId` struct
   - Added hit-test methods: `set_hit_test_mode`, `hit_test_mode`, `set_hit_regions`, `add_hit_region`, `remove_hit_region`, `clear_hit_regions`

2. **`tauri-runtime-wry` crate** (`crates/tauri-runtime-wry/src/lib.rs`)
   - Added `BringToFront` and `SendToBack` variants to `WebviewMessage` enum
   - Added hit-test message variants: `SetHitTestMode`, `HitTestMode`, `SetHitRegions`, `AddHitRegion`, `RemoveHitRegion`, `ClearHitRegions`
   - Implemented dispatcher methods that send these messages
   - Added message handlers that call WRY's API
   - Changed `wry` dependency to local path: `path = "../../../wry"`

3. **`tauri` crate** (`crates/tauri/src/webview/mod.rs`)
   - Exposed `bring_to_front()` and `send_to_back()` on `Webview` struct
   - Exposed hit-test methods on `Webview` struct
   - Re-exported `HitTestMode` and `HitRegionId`
   - Also added all methods to `WebviewWindow` in `webview_window.rs`

### Usage in Tauri App

```rust
// In a Tauri command - Z-order control
#[tauri::command]
async fn bring_tab_to_front(app: AppHandle, label: String) -> Result<(), String> {
    if let Some(webview) = app.get_webview(&label) {
        webview.bring_to_front().map_err(|e| e.to_string())
    } else {
        Err(format!("Webview '{}' not found", label))
    }
}

// Hit-test passthrough for overlay UIs
#[tauri::command]
async fn set_overlay_passthrough(app: AppHandle, label: String, enabled: bool) -> Result<(), String> {
    use tauri::webview::HitTestMode;
    if let Some(webview) = app.get_webview(&label) {
        let mode = if enabled { HitTestMode::PassThrough } else { HitTestMode::Normal };
        webview.set_hit_test_mode(mode).map_err(|e| e.to_string())
    } else {
        Err(format!("Webview '{}' not found", label))
    }
}
```

```typescript
// From TypeScript/React frontend
await invoke('bring_tab_to_front', { label: 'tab-1' });
await invoke('set_overlay_passthrough', { label: 'hud', enabled: true });
```

See the Tauri fork's `CLAUDE.md` for more details.

---

## Hit-Test Passthrough API (NEW)

Enables "holes" in webviews where mouse events pass through to views beneath. Essential for overlay UIs where a HUD webview sits on top but has transparent regions that shouldn't capture clicks.

### API

```rust
use wry::{HitTestMode, HitRegionId, Rect};

// Set hit-test mode
webview.set_hit_test_mode(HitTestMode::Normal)?;      // Default - capture all
webview.set_hit_test_mode(HitTestMode::RegionBased)?; // Only capture in regions
webview.set_hit_test_mode(HitTestMode::PassThrough)?; // All events pass through

// Define interactive regions (for RegionBased mode)
webview.set_hit_regions(vec![toolbar_rect, sidebar_rect])?;

// Dynamic region management
let id = webview.add_hit_region(dropdown_rect)?;  // Returns HitRegionId
webview.remove_hit_region(id)?;
webview.clear_hit_regions()?;

// Builder API
WebViewBuilder::new()
    .with_hit_test_mode(HitTestMode::RegionBased)
    .with_hit_regions(vec![...])
    .build_as_child(&window)?;
```

### Implementation Status

| Platform | Status | Implementation |
|----------|--------|----------------|
| **macOS** | ✅ Complete | `NSView.hitTest:` override |
| Windows | Stub | TODO |
| Linux | Stub | TODO |
| iOS | Stub | TODO |
| Android | No-op | Not supported |

### Files Changed

- `src/lib.rs` - `HitTestMode`, `HitRegionId` types + WebView/Builder methods
- `src/wkwebview/class/wry_web_view.rs` - ivars + `hitTest:` override
- `src/wkwebview/mod.rs` - InnerWebView implementation
- `src/webview2/mod.rs` - Windows stubs
- `src/webkitgtk/mod.rs` - Linux stubs
- `src/android/mod.rs` - Android no-ops
- `examples/hit_test_passthrough.rs` - Interactive test

### Test the Implementation

```bash
cargo run --example hit_test_passthrough
# Press M to toggle passthrough mode
# Click the overlap zone to see which webview receives clicks
```

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

- `examples/z_order_test.rs` - Interactive z-order test
- `examples/hit_test_passthrough.rs` - Hit-test passthrough demo
- `examples/snapshot_timing.rs` - Snapshot performance benchmark
- `examples/compositing_test.rs` - Original z-order research test

## Related Repositories

- **Tauri fork**: `/Volumes/Vault/Workspace/tauri` - Forked to add z-order support
- **eidolon**: `/Volumes/Vault/Workspace/eidolon` - Target application using this
