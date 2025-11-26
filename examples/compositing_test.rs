// Copyright 2020-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! Compositing Test Example
//!
//! This example tests the z-ordering problem between parent UI webviews and child content webviews.
//! The goal is to have dropdown menus from the "UI" layer render ABOVE the content webview.
//!
//! Current expected behavior: The dropdown will render BEHIND the content webview (the problem).
//! Desired behavior: The dropdown should render ABOVE the content webview.

use tao::{
  dpi::LogicalSize,
  event::{Event, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  window::WindowBuilder,
};
use wry::{Rect, WebViewBuilder};

const UI_HTML: &str = r#"
<!DOCTYPE html>
<html>
<head>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      background: transparent;
    }
    .toolbar {
      background: linear-gradient(to bottom, #3a3a3a, #2a2a2a);
      padding: 8px 16px;
      display: flex;
      align-items: center;
      gap: 12px;
      border-bottom: 1px solid #1a1a1a;
    }
    .menu-container {
      position: relative;
    }
    .menu-button {
      background: #4a4a4a;
      border: 1px solid #5a5a5a;
      border-radius: 4px;
      color: white;
      padding: 6px 16px;
      cursor: pointer;
      font-size: 13px;
    }
    .menu-button:hover {
      background: #5a5a5a;
    }
    .dropdown {
      display: none;
      position: absolute;
      top: 100%;
      left: 0;
      background: #3a3a3a;
      border: 1px solid #5a5a5a;
      border-radius: 4px;
      min-width: 200px;
      box-shadow: 0 4px 12px rgba(0,0,0,0.4);
      z-index: 9999;
      margin-top: 4px;
    }
    .dropdown.open {
      display: block;
    }
    .dropdown-item {
      padding: 10px 16px;
      color: white;
      cursor: pointer;
      border-bottom: 1px solid #4a4a4a;
    }
    .dropdown-item:hover {
      background: #5a5a5a;
    }
    .dropdown-item:last-child {
      border-bottom: none;
    }
    .content-hole {
      /* This is where the content webview will show through */
      margin: 16px;
      background: transparent;
      border: 2px dashed #5a5a5a;
      border-radius: 8px;
      height: calc(100vh - 100px);
      display: flex;
      align-items: center;
      justify-content: center;
      color: #666;
    }
    .status-bar {
      position: fixed;
      bottom: 0;
      left: 0;
      right: 0;
      background: #2a2a2a;
      padding: 4px 16px;
      font-size: 11px;
      color: #888;
      border-top: 1px solid #3a3a3a;
    }
    .fps-counter {
      position: fixed;
      top: 50px;
      right: 16px;
      background: rgba(0,0,0,0.8);
      color: #0f0;
      padding: 8px 12px;
      border-radius: 4px;
      font-family: monospace;
      font-size: 12px;
    }
  </style>
</head>
<body>
  <div class="toolbar">
    <div class="menu-container">
      <button class="menu-button" onclick="toggleMenu('file')">File</button>
      <div class="dropdown" id="file-menu">
        <div class="dropdown-item">New Tab</div>
        <div class="dropdown-item">New Window</div>
        <div class="dropdown-item">Open File...</div>
        <div class="dropdown-item">Save Page As...</div>
        <div class="dropdown-item">Print...</div>
      </div>
    </div>
    <div class="menu-container">
      <button class="menu-button" onclick="toggleMenu('edit')">Edit</button>
      <div class="dropdown" id="edit-menu">
        <div class="dropdown-item">Undo</div>
        <div class="dropdown-item">Redo</div>
        <div class="dropdown-item">Cut</div>
        <div class="dropdown-item">Copy</div>
        <div class="dropdown-item">Paste</div>
      </div>
    </div>
    <div class="menu-container">
      <button class="menu-button" onclick="toggleMenu('view')">View</button>
      <div class="dropdown" id="view-menu">
        <div class="dropdown-item">Zoom In</div>
        <div class="dropdown-item">Zoom Out</div>
        <div class="dropdown-item">Reset Zoom</div>
        <div class="dropdown-item">Full Screen</div>
        <div class="dropdown-item">Developer Tools</div>
      </div>
    </div>
  </div>

  <div class="content-hole">
    [Content WebView Area - Click dropdown menus to test z-ordering]
  </div>

  <div class="fps-counter" id="fps">FPS: --</div>

  <div class="status-bar">
    Press 'D' to toggle dropdown | Press 'T' to run snapshot timing test | Compositing Test v0.1
  </div>

  <script>
    let currentMenu = null;

    function toggleMenu(name) {
      const menu = document.getElementById(name + '-menu');
      if (currentMenu && currentMenu !== menu) {
        currentMenu.classList.remove('open');
      }
      menu.classList.toggle('open');
      currentMenu = menu.classList.contains('open') ? menu : null;
    }

    document.addEventListener('click', (e) => {
      if (!e.target.closest('.menu-container') && currentMenu) {
        currentMenu.classList.remove('open');
        currentMenu = null;
      }
    });

    // FPS counter animation loop
    let frameCount = 0;
    let lastTime = performance.now();

    function updateFPS() {
      frameCount++;
      const now = performance.now();
      if (now - lastTime >= 1000) {
        document.getElementById('fps').textContent = 'FPS: ' + frameCount;
        frameCount = 0;
        lastTime = now;
      }
      requestAnimationFrame(updateFPS);
    }
    updateFPS();

    // Keyboard shortcuts
    document.addEventListener('keydown', (e) => {
      if (e.key === 'd' || e.key === 'D') {
        toggleMenu('file');
      }
      if (e.key === 't' || e.key === 'T') {
        window.ipc.postMessage('run_snapshot_test');
      }
    });
  </script>
</body>
</html>
"#;

const CONTENT_HTML: &str = r#"
<!DOCTYPE html>
<html>
<head>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      background: linear-gradient(135deg, #1a1a2e 0%, #16213e 50%, #0f3460 100%);
      min-height: 100vh;
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      color: white;
    }
    h1 {
      font-size: 32px;
      margin-bottom: 16px;
    }
    .animation-box {
      width: 100px;
      height: 100px;
      background: linear-gradient(45deg, #e94560, #ff6b6b);
      border-radius: 12px;
      animation: spin 2s linear infinite, pulse 1s ease-in-out infinite alternate;
      margin: 24px 0;
    }
    @keyframes spin {
      from { transform: rotate(0deg); }
      to { transform: rotate(360deg); }
    }
    @keyframes pulse {
      from { transform: scale(1) rotate(0deg); }
      to { transform: scale(1.1) rotate(360deg); }
    }
    .info {
      background: rgba(255,255,255,0.1);
      padding: 16px 24px;
      border-radius: 8px;
      text-align: center;
      max-width: 400px;
    }
    .video-placeholder {
      width: 320px;
      height: 180px;
      background: #000;
      border-radius: 8px;
      margin-top: 24px;
      display: flex;
      align-items: center;
      justify-content: center;
      color: #666;
      font-size: 14px;
    }
    .fps-display {
      position: fixed;
      bottom: 16px;
      right: 16px;
      background: rgba(0,0,0,0.8);
      color: #0f0;
      padding: 8px 12px;
      border-radius: 4px;
      font-family: monospace;
    }
  </style>
</head>
<body>
  <h1>Content WebView</h1>
  <div class="animation-box"></div>
  <div class="info">
    <p>This is the content webview that should render BELOW the UI toolbar dropdowns.</p>
    <p style="margin-top: 8px; color: #888;">If you can see this text through an open dropdown menu, compositing is broken.</p>
  </div>
  <div class="video-placeholder">
    [Video would play here]
  </div>
  <div class="fps-display" id="fps">FPS: --</div>

  <script>
    let frameCount = 0;
    let lastTime = performance.now();

    function updateFPS() {
      frameCount++;
      const now = performance.now();
      if (now - lastTime >= 1000) {
        document.getElementById('fps').textContent = 'FPS: ' + frameCount;
        frameCount = 0;
        lastTime = now;
      }
      requestAnimationFrame(updateFPS);
    }
    updateFPS();
  </script>
</body>
</html>
"#;

fn main() -> wry::Result<()> {
  let event_loop = EventLoop::new();
  let window = WindowBuilder::new()
    .with_title("Compositing Test - WRY")
    .with_inner_size(LogicalSize::new(900.0, 700.0))
    .build(&event_loop)
    .unwrap();

  let size = window.inner_size().to_logical::<u32>(window.scale_factor());
  let toolbar_height = 45u32;

  // Create the content webview FIRST (should be behind)
  // Position it below the toolbar area
  let content_builder = WebViewBuilder::new()
    .with_bounds(Rect {
      position: wry::dpi::LogicalPosition::new(16, toolbar_height + 16).into(),
      size: wry::dpi::LogicalSize::new(size.width - 32, size.height - toolbar_height - 48).into(),
    })
    .with_html(CONTENT_HTML)
    .with_transparent(true);

  // Create the UI webview SECOND (should be on top, but won't be due to the bug)
  let ui_builder = WebViewBuilder::new()
    .with_bounds(Rect {
      position: wry::dpi::LogicalPosition::new(0, 0).into(),
      size: wry::dpi::LogicalSize::new(size.width, size.height).into(),
    })
    .with_html(UI_HTML)
    .with_transparent(true)
    .with_ipc_handler(|req| {
      let body = req.body();
      if body == "run_snapshot_test" {
        println!("Snapshot test requested via IPC");
        // TODO: Implement snapshot timing test
      }
    });

  // Platform-specific webview building
  #[cfg(any(
    target_os = "windows",
    target_os = "macos",
    target_os = "ios",
    target_os = "android"
  ))]
  let (content_webview, ui_webview) = {
    let content = content_builder.build_as_child(&window)?;
    let ui = ui_builder.build_as_child(&window)?;
    (content, ui)
  };

  #[cfg(not(any(
    target_os = "windows",
    target_os = "macos",
    target_os = "ios",
    target_os = "android"
  )))]
  let (content_webview, ui_webview) = {
    use tao::platform::unix::WindowExtUnix;
    use wry::WebViewBuilderExtUnix;
    let vbox = window.default_vbox().unwrap();
    // Note: On Linux with GTK, child webviews work differently
    // For this test, we'll use the main webview approach
    let content = content_builder.build_gtk(vbox)?;
    let ui = ui_builder.build_gtk(vbox)?;
    (content, ui)
  };

  println!("=== Compositing Test ===");
  println!("Content webview created first (should be behind)");
  println!("UI webview created second (should be on top)");
  println!();
  println!("TEST: Click 'File' menu - the dropdown should appear ABOVE the animated content.");
  println!("EXPECTED BUG: Dropdown appears BEHIND the content webview.");
  println!();
  println!("Press 'D' to toggle File dropdown");
  println!("Press 'T' to run snapshot timing test");

  event_loop.run(move |event, _, control_flow| {
    *control_flow = ControlFlow::Wait;

    match event {
      Event::WindowEvent {
        event: WindowEvent::Resized(size),
        ..
      } => {
        let size = size.to_logical::<u32>(window.scale_factor());
        let toolbar_height = 45u32;

        // Resize content webview
        let _ = content_webview.set_bounds(Rect {
          position: wry::dpi::LogicalPosition::new(16, toolbar_height + 16).into(),
          size: wry::dpi::LogicalSize::new(size.width - 32, size.height - toolbar_height - 48)
            .into(),
        });

        // Resize UI webview
        let _ = ui_webview.set_bounds(Rect {
          position: wry::dpi::LogicalPosition::new(0, 0).into(),
          size: wry::dpi::LogicalSize::new(size.width, size.height).into(),
        });
      }
      Event::WindowEvent {
        event: WindowEvent::CloseRequested,
        ..
      } => {
        *control_flow = ControlFlow::Exit;
      }
      _ => {}
    }
  });
}
