# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Wry is a cross-platform WebView rendering library for Rust, part of the Tauri ecosystem. It provides a unified API to create webviews using platform-native web engines:
- **macOS/iOS**: WKWebView (WebKit)
- **Windows**: WebView2 (Edge Chromium)
- **Linux**: WebKitGTK
- **Android**: Android WebView

## Build Commands

```bash
# Build the library
cargo build

# Build with all features
cargo build --all-features

# Run tests
cargo test

# Run a specific example (uses tao windowing library)
cargo run --example simple
cargo run --example multiwebview

# Format code (uses 2-space indentation)
cargo fmt

# Run clippy
cargo clippy
```

## Key Feature Flags

- `os-webview` (default): Enables platform WebView framework
- `protocol` (default): Custom URL scheme handling via `with_custom_protocol`
- `drag-drop` (default): File drag-drop handling
- `devtools`: Enable devtools in release builds (uses private APIs on macOS - avoid for App Store)
- `transparent`: Transparent backgrounds (private APIs on macOS)
- `fullscreen`: Fullscreen media (private APIs on macOS)
- `linux-body`: Request body support on Linux (requires WebKitGTK 2.40+)
- `x11`: X11 support on Linux (default)

## Architecture

### Platform Implementations

Each platform has its own `InnerWebView` implementation in `src/`:

- `src/wkwebview/` - macOS and iOS (WKWebView)
  - `class/` - Objective-C class wrappers (delegates, handlers)
  - `ios/` - iOS-specific extensions including `WKWebView.rs` bindings
- `src/webview2/` - Windows (WebView2/Edge Chromium)
- `src/webkitgtk/` - Linux (WebKitGTK)
  - `web_context.rs` - WebContext management for data isolation
- `src/android/` - Android
  - `binding.rs` - JNI bindings
  - `main_pipe.rs` - Communication pipe for Android WebView

### Core Types (in `src/lib.rs`)

- `WebView` - Main webview handle, wraps platform-specific `InnerWebView`
- `WebViewBuilder` - Builder pattern for creating webviews
- `WebViewAttributes` - Configuration options (URL, scripts, handlers, etc.)

### Platform Extensions

- `WebViewBuilderExtUnix` - Linux-specific methods (`build_gtk`, `new_gtk`)
- `WebViewBuilderExtWindows` - Windows-specific methods
- `WebViewBuilderExtDarwin` - macOS/iOS-specific methods
- `WebViewBuilderExtMacos` - macOS-only (`with_webview_configuration`)
- `WebViewBuilderExtIos` - iOS-only methods

## Code Style

- 2-space indentation (see `rustfmt.toml`)
- Max line width: 100 characters
- Imports grouped by crate (`imports_granularity = "Crate"`)
- License header required on source files (Apache-2.0 OR MIT)

## Release Process

Uses [covector](https://github.com/jbolda/covector) for changelog management. Change files go in `.changes/` directory.

---

## Compositing Research Focus

This fork is being used to investigate webview compositing for the eidolon project (../eidolon). The goal is to enable UI elements (dropdowns, modals, overlays) to render **above** child webviews, which is not possible with the current native window hierarchy.

### Key Investigation Areas

#### macOS Backend (`src/wkwebview/`)

**View Hierarchy Creation** - `src/wkwebview/mod.rs:195-658`
- `new_ns_view()` creates the webview and attaches it to the window
- Child webviews: `ns_view.addSubview(&webview)` (line 617)
- Parent webviews: Creates `WryWebViewParent` wrapper, sets as `setContentView` (lines 619-639)
- The `WryWebView` extends `WKWebView` - see `src/wkwebview/class/wry_web_view.rs`

**WryWebViewParent** - `src/wkwebview/class/wry_web_view_parent.rs`
- Custom NSView subclass that wraps the webview
- Handles traffic light positioning and key events
- This is where z-ordering modifications could potentially happen

**Snapshot API** - `src/wkwebview/ios/WKWebView.rs:366-371`
- `takeSnapshotWithConfiguration_completionHandler` - existing binding for WKWebView snapshots
- Returns NSImage, could be used for offscreen rendering approach
- Performance concern: ~30-40ms per frame

**Potential Modification Points:**
1. `WryWebViewParent` - could add overlay NSView above webview
2. `new_ns_view()` - could intercept CALayer or create transparent overlay window
3. Could add new methods to access underlying layer for compositing

#### Windows Backend (`src/webview2/mod.rs`)

**Controller Creation** - `src/webview2/mod.rs:363-400`
- Uses `CreateCoreWebView2ControllerCompletedHandler`
- Currently uses windowed hosting mode
- **Key opportunity**: Could switch to `CreateCoreWebView2CompositionController` for visual hosting

**Environment Setup** - `src/webview2/mod.rs:281-361`
- `CreateCoreWebView2EnvironmentWithOptions` - where composition mode could be configured

**Container HWND** - `src/webview2/mod.rs:180-279`
- Creates a child window for the webview
- Z-ordering via `SetWindowPos` with `HWND_TOP`

### Approach Options (from COMPOSITING_RESEARCH.md)

1. **Inverted z-order**: Make parent webview transparent, render ON TOP of child webviews
2. **Platform window APIs**: Force z-ordering via NSWindow level / Win32 z-order
3. **Transparent overlay window**: Separate window for UI only, positioned above
4. **Visual hosting (Windows)**: Switch to `CoreWebView2CompositionController` for texture capture
5. **Snapshot compositing**: Use `takeSnapshot` APIs to render webviews to textures

### Related Files for Investigation

- `examples/multiwebview.rs` - Multi-webview example to test with
- `examples/transparent.rs` - Transparency example
- `examples/wgpu.rs` - GPU integration example (relevant for texture compositing)
- `examples/compositing_test.rs` - **NEW** Tests z-ordering between UI and content webviews
- `examples/snapshot_timing.rs` - **NEW** Measures WKWebView snapshot latency

---

## Compositing Session Progress (Nov 2024)

### Target Application

**eidolon** (`../eidolon`) - A multi-tab web browser built with Tauri that uses separate webviews for content in each tab. The problem: UI elements (modals, dropdowns, message boxes) render UNDER the content webviews instead of above them.

### Code Changes Made

#### 1. Cargo.toml - Added snapshot dependencies
```toml
# objc2-web-kit features:
"WKSnapshotConfiguration"

# objc2-app-kit features (macOS):
"NSImage"
"NSBitmapImageRep"
```

#### 2. `src/wkwebview/mod.rs` - Added take_snapshot() implementation
Location: Lines 1216-1270 (after `reparent()` method)

```rust
#[cfg(target_os = "macos")]
pub fn take_snapshot<F: FnOnce(std::time::Duration, Option<Vec<u8>>) + Send + 'static>(
  &self,
  cb: F,
)
```

- Creates `WKSnapshotConfiguration` for full-view capture
- Uses `block2::RcBlock` for async callback handling
- Converts `NSImage` to TIFF bytes via `TIFFRepresentation()`
- Returns elapsed `Duration` and optional image data

#### 3. `src/lib.rs` - Exposed take_snapshot in public API
Location: Lines 2376-2421

- Added `take_snapshot()` to `WebViewExtMacOS` trait
- Added implementation that delegates to `InnerWebView`
- Includes documentation with usage example

### Test Examples Created

**`examples/compositing_test.rs`** (NEW FILE)
- Two sibling WKWebViews: content webview + transparent UI overlay webview
- UI webview created SECOND so it's on top in NSView z-order
- Has toolbar with dropdown menus to test if UI appears above content
- Run: `cargo run --example compositing_test --features transparent`

**`examples/snapshot_timing.rs`** (NEW FILE)
- Measures WKWebView `takeSnapshot()` latency
- Uses IPC channel pattern for keyboard handling (webview captures focus)
- Press T for snapshot, S for statistics, C to clear
- Run: `cargo run --example snapshot_timing`

### Key Findings

#### 1. Inverted Z-Order Works!
The compositing_test revealed that **sibling webviews DO respect z-order**:
- Content webview added first (lower z-index)
- UI webview added second with `with_transparent(true)` (higher z-index)
- Dropdowns from UI webview correctly appear ABOVE content webview

This means the "inverted z-order" approach is viable for eidolon!

#### 2. Snapshot Performance is Excellent
Timing results on macOS (800x600 window, simple content):
```
Average: 1.37ms
Min:     1.29ms
Max:     1.72ms
Theoretical FPS: ~700+
60fps capable: YES (well under 16.67ms budget)
```

Note: This measures API call time, not GPU texture upload. Real-world performance with complex content and larger windows needs testing.

#### 3. macOS View Hierarchy Understanding
- `build_as_child(&window)` creates sibling NSViews via `addSubview`
- Later `addSubview` calls place views ON TOP
- Transparent backgrounds let underlying content show through

### Implications for eidolon

**Option A: Inverted Z-Order (Recommended)**
- Make the Tauri UI webview render ON TOP of content webviews
- UI webview has transparent background with "holes" where content shows
- Content webviews are siblings positioned below UI webview
- No WRY modifications needed beyond ensuring correct creation order

**Option B: Snapshot Compositing (If Option A fails)**
- Use `take_snapshot()` to capture content webviews as textures
- Composite in wgpu/GPU with proper z-ordering
- More complex but enables advanced effects (blur, shadows, transitions)

### Next Steps

1. **Test inverted z-order in eidolon** - Modify tab/webview creation order
2. **Test with complex content** - Video playback, WebGL, animations
3. **Larger window testing** - 4K displays, multiple monitors
4. **Windows investigation** - WebView2 CompositionController for visual hosting
5. **Consider hybrid approach** - Inverted z-order + snapshot for special effects

### Files Modified (uncommitted)
- `Cargo.toml` - Added WKSnapshotConfiguration, NSImage, NSBitmapImageRep features
- `src/lib.rs` - Added take_snapshot to WebViewExtMacOS trait
- `src/wkwebview/mod.rs` - Added take_snapshot implementation

### Files Created (untracked)
- `examples/compositing_test.rs` - Z-ordering test with two webviews
- `examples/snapshot_timing.rs` - Snapshot latency benchmarking
