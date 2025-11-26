# WRY Z-Order API Implementation - Session Prompt

## Context

We are working on a fork of **WRY** (Tauri's cross-platform webview library) to add dynamic z-order control for webviews within a window.

### The Problem

When using WRY to create multiple webviews within a single window, the z-ordering is determined by creation order and cannot be changed afterwards. Child webviews always render on top of earlier webviews, so UI elements (modals, dropdowns, overlays) from a parent/earlier webview are hidden behind content webviews.

```
Current limitation:
┌─────────────────────────────────────┐
│ Window                              │
│ ├─ Webview A (created first)        │ ← Always at bottom
│ │   └─ UI overlay trying to show... │ ← HIDDEN!
│ └─ Webview B (created later)        │ ← Always on top, blocks A
└─────────────────────────────────────┘
```

### What We've Tried

1. **Inverted Z-Order Approach** - Made UI webview transparent and rendered it on top of content webviews.
   - **Result**: Z-ordering worked in isolation (proven in `examples/compositing_test.rs`), but input passthrough was a fatal flaw - the transparent top layer captured all mouse events, preventing interaction with webviews underneath.

2. **Offscreen Rendering / Snapshot Compositing** - Considered rendering webviews to textures and compositing in canvas/wgpu.
   - **Result**: Too complex. Would require continuous snapshot streaming, image transfer pipeline, full input forwarding, and would break text selection, video, etc. We did prove snapshots are fast (~1.4ms) in `examples/snapshot_timing.rs`, but the overall approach is not viable.

### The Solution We Want to Pursue

**Dynamic Z-Order API** - Add APIs to WRY for controlling webview z-order at runtime:

```rust
// Individual webview control
webview.bring_to_front()  // Move this webview to top of z-order
webview.send_to_back()    // Move this webview to bottom of z-order

// Layer-based grouping (webviews in same layer share z-level)
webview.set_z_layer(0)    // Assign to layer 0
webview.set_z_layer(1)    // Assign to layer 1 (higher = on top)

// Batch operations for layer groups
manager.bring_layer_to_front(0)  // Bring all layer-0 webviews to front
manager.set_layer_order([0, 1, 2])  // Explicit ordering of layers
```

**Use cases:**
- Multiple content webviews in tiled/grid layouts should share the same z-level
- A UI webview needs to dynamically move above content webviews when showing overlays
- Individual webviews may need to be brought forward (e.g., focus indication)

```
Desired capability:
┌─────────────────────────────────────┐
│ Layer 1 (when active)               │
│ └─ UI Webview                       │
│     └─ Modals, dropdowns visible!   │
├─────────────────────────────────────┤
│ Layer 0                             │
│ ├─ Content WV 1 ─┬─ Content WV 2    │  ← All at same z-level
│ ├─ Content WV 3 ─┴─ Content WV 4    │  ← Tiled layout
│ └─ (can reorder within layer too)   │
└─────────────────────────────────────┘
```

**Key requirements:**
- Move individual webviews forward/backward in z-order
- Group webviews into layers that share the same z-level
- Move entire layers forward/backward as a group
- Dynamic changes at runtime (not just at creation)

**Why this approach:**
- No transparency complexity
- No input passthrough / hit-testing needed
- Works with existing native webview architecture
- Flexible: supports both individual and batch z-order changes

## Repository State

### Files Modified in WRY fork (uncommitted)
- `Cargo.toml` - Added WKSnapshotConfiguration, NSImage, NSBitmapImageRep features
- `src/lib.rs` - Added `take_snapshot()` to `WebViewExtMacOS` trait
- `src/wkwebview/mod.rs` - Added `take_snapshot()` implementation

### Files Created (untracked)
- `examples/compositing_test.rs` - Two sibling webviews, proves z-ordering works
- `examples/snapshot_timing.rs` - Measures snapshot latency (~1.4ms average)
- `CLAUDE.md` - Session documentation
- `NEXT_SESSION_PROMPT.md` - This file

### Key Source Locations
- `src/wkwebview/mod.rs` - macOS webview implementation, where z-order APIs will be added
- `src/wkwebview/class/wry_web_view.rs` - WryWebView class (extends WKWebView)
- `src/lib.rs` - Public API, `WebViewExtMacOS` trait where we'll expose new methods
- `src/webview2/mod.rs` - Windows webview implementation

## Task for Next Session

**Create a detailed implementation plan for adding z-order control to WRY:**

1. **Design the API:**
   - Individual control: `bring_to_front()`, `send_to_back()`, `set_z_index()`
   - Layer grouping: `set_z_layer(layer_id)`
   - Batch operations: How to move all webviews in a layer together
   - Where state is stored (per-webview, global manager, or both)

2. **Research platform APIs:**
   - macOS: NSView `addSubview:positioned:relativeTo:`, `sortSubviewsUsingFunction:context:`
   - Windows: `SetWindowPos` with `HWND_TOP`/`HWND_BOTTOM`, or z-order flags
   - How to maintain layer groupings when z-order changes

3. **Implementation locations:**
   - `InnerWebView` methods in `src/wkwebview/mod.rs` (macOS)
   - `InnerWebView` methods in `src/webview2/mod.rs` (Windows)
   - Public API in `src/lib.rs` - cross-platform trait or platform-specific extensions
   - Layer manager: new module or integrate into existing structures

4. **Consider edge cases:**
   - Webview with no superview/parent
   - Webviews in different windows
   - Adding new webview to existing layer
   - Removing webview from layer
   - Performance of frequent z-order changes

5. **Plan Tauri integration:**
   - Commands to expose: `set_webview_z_layer`, `bring_webview_to_front`, `bring_layer_to_front`
   - How apps will coordinate z-order changes with UI state

## Related Documentation

- `CLAUDE.md` in this repo - Full session progress and findings
- `COMPOSITING_RESEARCH.md` - Original research notes

## Commands

```bash
# Build WRY
cargo build

# Run compositing test (shows z-ordering works with sibling webviews)
cargo run --example compositing_test --features transparent

# Run snapshot timing test
cargo run --example snapshot_timing

# Check current changes
git status
git diff
```
