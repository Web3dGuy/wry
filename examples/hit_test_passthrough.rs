// Copyright 2020-2024 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! Hit-Test Passthrough Example
//!
//! Demonstrates the hit-test passthrough feature using TWO SIDE-BY-SIDE webviews
//! that PARTIALLY OVERLAP in the center. This makes it easy to see which webview
//! is receiving clicks.
//!
//! ## Layout:
//! ```text
//! ┌──────────────────────────────────────────────┐
//! │  ┌─── RED (left) ────┐ ┌─── BLUE (right) ──┐ │
//! │  │                   │ │                   │ │
//! │  │  Click counter: 0 │ │  Click counter: 0 │ │
//! │  │                   │ │                   │ │
//! │  │              ▼────┼─┼────▲              │ │
//! │  │          OVERLAP  │ │  ZONE             │ │
//! │  │              ▲────┼─┼────▼              │ │
//! │  │                   │ │                   │ │
//! │  └───────────────────┘ └───────────────────┘ │
//! └──────────────────────────────────────────────┘
//! ```
//!
//! ## Test:
//! - The BLUE webview is on top (created second)
//! - In NORMAL mode: Clicking the overlap zone increments BLUE counter
//! - In PASSTHROUGH mode: Clicking the overlap zone increments RED counter
//!
//! ## Controls:
//! - **M**: Toggle between Normal and PassThrough mode
//! - **Q** or **Escape**: Quit

use winit::{
  application::ApplicationHandler,
  event::{ElementState, WindowEvent},
  event_loop::{ActiveEventLoop, EventLoop},
  keyboard::{KeyCode, PhysicalKey},
  window::{Window, WindowId},
};
use wry::{
  dpi::{LogicalPosition, LogicalSize},
  HitTestMode, Rect, WebViewBuilder,
};

const WINDOW_WIDTH: u32 = 700;
const WINDOW_HEIGHT: u32 = 400;

struct State {
  window: Option<Window>,
  red_webview: Option<wry::WebView>,
  blue_webview: Option<wry::WebView>,
  passthrough_enabled: bool,
}

impl Default for State {
  fn default() -> Self {
    Self {
      window: None,
      red_webview: None,
      blue_webview: None,
      passthrough_enabled: false,
    }
  }
}

impl ApplicationHandler for State {
  fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    let mut attributes = Window::default_attributes();
    attributes.inner_size = Some(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT).into());
    attributes.title = "Hit-Test Demo - Press M to toggle passthrough, Q to quit".to_string();
    let window = event_loop.create_window(attributes).unwrap();

    // Create RED webview (LEFT side) - created first, so it's behind
    let red_webview = WebViewBuilder::new()
      .with_html(
        r#"<!DOCTYPE html>
<html>
<head>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body {
      background: #e74c3c;
      height: 100vh;
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      font-family: -apple-system, system-ui, sans-serif;
      cursor: pointer;
      user-select: none;
    }
    body:active { background: #c0392b; }
    h1 { color: white; font-size: 24px; margin-bottom: 10px; }
    #counter { font-size: 64px; color: white; }
    .label { color: rgba(255,255,255,0.7); font-size: 14px; margin-top: 20px; }
  </style>
</head>
<body onclick="increment()">
  <h1>RED (Behind)</h1>
  <div id="counter">0</div>
  <div class="label">Click anywhere</div>
  <script>
    let count = 0;
    function increment() {
      count++;
      document.getElementById('counter').textContent = count;
    }
  </script>
</body>
</html>"#,
      )
      // Position: Left side, extending into center overlap zone
      .with_bounds(Rect {
        position: LogicalPosition::new(20, 50).into(),
        size: LogicalSize::new(400, 300).into(), // Extends to x=420
      })
      .build_as_child(&window)
      .unwrap();

    // Create BLUE webview (RIGHT side) - created second, so it's on top
    let blue_webview = WebViewBuilder::new()
      .with_html(
        r#"<!DOCTYPE html>
<html>
<head>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body {
      background: #3498db;
      height: 100vh;
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      font-family: -apple-system, system-ui, sans-serif;
      cursor: pointer;
      user-select: none;
    }
    body:active { background: #2980b9; }
    h1 { color: white; font-size: 24px; margin-bottom: 10px; }
    #counter { font-size: 64px; color: white; }
    .label { color: rgba(255,255,255,0.7); font-size: 14px; margin-top: 20px; }
    #mode {
      position: fixed;
      top: 10px;
      left: 50%;
      transform: translateX(-50%);
      background: rgba(0,0,0,0.7);
      color: white;
      padding: 8px 16px;
      border-radius: 6px;
      font-size: 13px;
    }
  </style>
</head>
<body onclick="increment()">
  <div id="mode">Mode: NORMAL (Blue on top)</div>
  <h1>BLUE (On Top)</h1>
  <div id="counter">0</div>
  <div class="label">Click anywhere</div>
  <script>
    let count = 0;
    function increment() {
      count++;
      document.getElementById('counter').textContent = count;
    }
    window.setMode = function(mode) {
      document.getElementById('mode').textContent = 'Mode: ' + mode;
    }
  </script>
</body>
</html>"#,
      )
      // Position: Right side, starting from center overlap zone
      .with_bounds(Rect {
        position: LogicalPosition::new(280, 50).into(), // Starts at x=280, overlaps 280-420
        size: LogicalSize::new(400, 300).into(),
      })
      .build_as_child(&window)
      .unwrap();

    // Blue is already on top since it was created second
    // (but we can use bring_to_front to be explicit)
    let _ = blue_webview.bring_to_front();

    println!("Hit-Test Passthrough Demo");
    println!("=========================");
    println!();
    println!("Layout: RED (left) and BLUE (right) webviews OVERLAP in the center");
    println!("BLUE is on top (created second)");
    println!();
    println!("TEST:");
    println!("  1. Click the OVERLAP ZONE (center) - BLUE counter should increment");
    println!("  2. Press M to enable PASSTHROUGH");
    println!("  3. Click the OVERLAP ZONE again - RED counter should increment!");
    println!();
    println!("Current mode: NORMAL (clicks go to BLUE)");

    self.window = Some(window);
    self.red_webview = Some(red_webview);
    self.blue_webview = Some(blue_webview);
  }

  fn window_event(
    &mut self,
    event_loop: &ActiveEventLoop,
    _window_id: WindowId,
    event: WindowEvent,
  ) {
    match event {
      WindowEvent::CloseRequested => {
        event_loop.exit();
      }
      WindowEvent::KeyboardInput { event, .. } => {
        if event.state == ElementState::Pressed {
          match event.physical_key {
            PhysicalKey::Code(KeyCode::KeyM) => {
              if let Some(blue) = &self.blue_webview {
                self.passthrough_enabled = !self.passthrough_enabled;

                if self.passthrough_enabled {
                  let _ = blue.set_hit_test_mode(HitTestMode::PassThrough);
                  let _ = blue.evaluate_script("window.setMode && setMode('PASSTHROUGH (clicks go through to RED)')");
                  println!("Mode: PASSTHROUGH - clicks pass through BLUE to RED");
                } else {
                  let _ = blue.set_hit_test_mode(HitTestMode::Normal);
                  let _ = blue.evaluate_script("window.setMode && setMode('NORMAL (Blue on top)')");
                  println!("Mode: NORMAL - BLUE captures all clicks");
                }
              }
            }

            PhysicalKey::Code(KeyCode::KeyQ) | PhysicalKey::Code(KeyCode::Escape) => {
              event_loop.exit();
            }
            _ => {}
          }
        }
      }
      _ => {}
    }
  }

  fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
    #[cfg(any(
      target_os = "linux",
      target_os = "dragonfly",
      target_os = "freebsd",
      target_os = "netbsd",
      target_os = "openbsd",
    ))]
    {
      while gtk::events_pending() {
        gtk::main_iteration_do(false);
      }
    }
  }
}

fn main() -> wry::Result<()> {
  #[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
  ))]
  {
    use gtk::prelude::DisplayExtManual;

    gtk::init()?;
    if gtk::gdk::Display::default().unwrap().backend().is_wayland() {
      panic!("This example doesn't support wayland!");
    }

    winit::platform::x11::register_xlib_error_hook(Box::new(|_display, error| {
      let error = error as *mut x11_dl::xlib::XErrorEvent;
      (unsafe { (*error).error_code }) == 170
    }));
  }

  let event_loop = EventLoop::new().unwrap();
  let mut state = State::default();
  event_loop.run_app(&mut state).unwrap();

  Ok(())
}
