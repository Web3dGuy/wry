# WRY Webview Compositing Research

## The Problem

When using WRY to create multiple webviews within a single window (a parent webview + child webviews), the child webviews **always render on top** of the parent webview's UI. This is a fundamental limitation of how native webviews work - they exist in the OS window hierarchy, not as DOM elements.

### Visual Representation

```
┌─────────────────────────────────────────┐
│ Native Window                           │
│ ┌─────────────────────────────────────┐ │
│ │ Parent Webview (App UI)             │ │
│ │ - Renders menus, toolbars, overlays │ │
│ │ - Dropdowns, modals, tooltips       │ │
│ └─────────────────────────────────────┘ │
│ ┌───────────┐ ┌───────────┐            │
│ │ Child WV  │ │ Child WV  │  ← Always  │
│ │ (content) │ │ (content) │    on top! │
│ └───────────┘ └───────────┘            │
└─────────────────────────────────────────┘
```

### Goal

Find a way to embed child webviews within a parent webview's UI such that:
- UI elements (dropdowns, modals, overlays) can render **above** child webviews
- OR child webviews render to a compositable surface that respects z-ordering
- Maintain reasonable performance (60fps target)

---

## Prior Research

### Known Constraints

1. **OS-level limitation** - Webviews are native windows/views, not DOM elements
2. **Electron's solution** - Stack Browser made ALL UI elements into separate BrowserViews positioned on top
3. **WRY status** - No offscreen rendering support (open feature request: Issue #391)

### Platform-Specific Possibilities

| Platform | API | Approach | Notes |
|----------|-----|----------|-------|
| **macOS** | WKWebView | `takeSnapshot()` | Apple blocks true offscreen rendering; ~30-40ms/frame |
| **macOS** | CALayer/IOSurface | Layer capture | Private APIs, may work |
| **Windows** | WebView2 | `CoreWebView2CompositionController` | Visual hosting mode, capture to D3D11 texture |
| **Windows** | GraphicsCaptureItem | Screen capture API | GPU-accelerated |
| **Linux** | WebKitGTK | `webkit_web_view_get_snapshot()` | Cairo surface output |

### Performance Concerns

- macOS snapshot: ~30-40ms per frame, high CPU for texture conversion
- Windows composition capture: Unknown, but GPU-accelerated
- Any compositing adds overhead vs direct rendering

---

## WRY Source Structure

```
src/
├── wkwebview/     # macOS/iOS - WKWebView wrapper
├── webview2/      # Windows - WebView2/Chromium wrapper
├── webkitgtk/     # Linux - WebKitGTK wrapper
├── android/       # Android
├── lib.rs         # Unified public API (~98KB)
├── web_context.rs # Shared context
```

---

## Investigation Tasks

### 1. macOS Backend (`src/wkwebview/`)

- [ ] How is WKWebView created and attached to the window?
- [ ] What's the relationship between the webview and its parent NSView?
- [ ] Is there access to the underlying CALayer?
- [ ] Can we intercept the rendering pipeline before it hits the screen?
- [ ] What private APIs might help (IOSurface, CARenderer)?
- [ ] How does `takeSnapshot` work internally?
- [ ] Can we control NSView z-ordering within the window?

### 2. Windows Backend (`src/webview2/`)

- [ ] How is WebView2 controller created?
- [ ] Is `CreateCoreWebView2CompositionController` available or easy to add?
- [ ] What would switching from windowed to visual hosting require?
- [ ] Can we access underlying DirectComposition visuals?

### 3. Alternative Approaches

- [ ] **Inverted z-order**: Make parent webview transparent, render it ON TOP of child webviews
- [ ] **Platform window APIs**: Force z-ordering via NSWindow level / Win32 z-order
- [ ] **Transparent overlay window**: Separate window for UI only, positioned above
- [ ] **Webviews as separate windows**: Composite at window manager level
- [ ] **All UI as webviews**: Stack Browser approach - make overlays into child webviews

### 4. Questions to Answer

1. **Feasibility**: Is texture-based compositing achievable without massive WRY changes?
2. **Performance**: Realistic performance impact of each approach?
3. **Minimum viable**: Smallest change to get UI rendering above webviews?
4. **Cross-platform**: Solution that works on both macOS and Windows?

---

## Reference Materials

### GitHub Issues
- [WRY #391 - Offscreen rendering feature](https://github.com/tauri-apps/wry/issues/391)
- [WRY #443 - BrowserView-like embedding](https://github.com/tauri-apps/wry/issues/443)
- [Tauri #11944 - wgpu overlay discussion](https://github.com/tauri-apps/tauri/discussions/11944)
- [Tauri #8246 - WebView on GPU content](https://github.com/tauri-apps/tauri/issues/8246)

### Working Examples
- [WebView2 D3D11 Texture Capture (Windows)](https://gist.github.com/pabloko/5b5bfb71ac52d20dfad714c666a0c428)
- [WKWebView to NSImage (macOS)](https://stackoverflow.com/questions/30149373/render-off-screen-wkwebview-into-nsimage)

### Documentation
- [WebView2 Visual vs Windowed Hosting](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/windowed-vs-visual-hosting)
- [Apple WKWebView.takeSnapshot](https://developer.apple.com/documentation/webkit/wkwebview/2873260-takesnapshot)

### Related Projects
- [Stack Browser Architecture](https://www.ika.im/posts/building-a-browser-in-electron) - All UI as BrowserViews

---

## Session Approach

1. **Start with macOS backend** (`src/wkwebview/`) - primary dev environment
2. **Read through the module** to understand webview lifecycle
3. **Identify modification points** for compositing injection
4. **Document findings** with specific `file:line` references
5. **Assess effort** for each potential approach
6. **Create proof-of-concept plan** for most promising approach

---

## Success Criteria

- [ ] Clear understanding of WRY's webview lifecycle on macOS
- [ ] Identified 2-3 potential approaches with pros/cons
- [ ] Rough effort estimate for each approach
- [ ] Decision on which approach to prototype first
- [ ] Initial POC plan documented

---

## Development Environment

- **Primary**: macOS
- **Secondary**: Windows (available for testing)
