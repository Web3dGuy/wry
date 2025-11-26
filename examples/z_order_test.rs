// Copyright 2020-2024 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! Z-Order Test Example
//!
//! Creates two overlapping webviews and allows toggling which is on top.
//!
//! ## Controls:
//! - **T**: Toggle z-order (swap which webview is on top)
//! - **F**: Bring content (red) webview to front
//! - **B**: Send content (red) webview to back
//! - **Q** or **Escape**: Quit
//!
//! ## Expected Behavior:
//! - Initially, the blue (UI) webview should be on top since it was created second
//! - Pressing T should swap the z-order
//! - The webview in front should be fully visible in the overlapping region

use winit::{
  application::ApplicationHandler,
  event::{ElementState, WindowEvent},
  event_loop::{ActiveEventLoop, EventLoop},
  keyboard::{KeyCode, PhysicalKey},
  window::{Window, WindowId},
};
use wry::{
  dpi::{LogicalPosition, LogicalSize},
  Rect, WebViewBuilder,
};

struct State {
  window: Option<Window>,
  content_webview: Option<wry::WebView>,
  ui_webview: Option<wry::WebView>,
  content_on_top: bool,
}

impl Default for State {
  fn default() -> Self {
    Self {
      window: None,
      content_webview: None,
      ui_webview: None,
      content_on_top: false, // UI webview starts on top (created second)
    }
  }
}

impl ApplicationHandler for State {
  fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    let mut attributes = Window::default_attributes();
    attributes.inner_size = Some(LogicalSize::new(600, 450).into());
    attributes.title = "Z-Order Test - Press T to toggle, Q to quit".to_string();
    let window = event_loop.create_window(attributes).unwrap();

    // Create content webview (red, created first = initially behind)
    let content_webview = WebViewBuilder::new()
      .with_html(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
          <style>
            body {
              background: #e74c3c;
              margin: 0;
              display: flex;
              align-items: center;
              justify-content: center;
              height: 100vh;
              font-family: system-ui, -apple-system, sans-serif;
            }
            .container {
              text-align: center;
              color: white;
            }
            h1 { margin: 0; font-size: 24px; }
            p { margin: 10px 0 0 0; opacity: 0.8; font-size: 14px; }
          </style>
        </head>
        <body>
          <div class="container">
            <h1>Content WebView</h1>
            <p>Red - Created First</p>
          </div>
        </body>
        </html>
      "#,
      )
      .with_bounds(Rect {
        position: LogicalPosition::new(50, 50).into(),
        size: LogicalSize::new(350, 250).into(),
      })
      .build_as_child(&window)
      .unwrap();

    // Create UI webview (blue, created second = initially on top)
    let ui_webview = WebViewBuilder::new()
      .with_html(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
          <style>
            body {
              background: #3498db;
              margin: 0;
              display: flex;
              align-items: center;
              justify-content: center;
              height: 100vh;
              font-family: system-ui, -apple-system, sans-serif;
            }
            .container {
              text-align: center;
              color: white;
            }
            h1 { margin: 0; font-size: 24px; }
            p { margin: 10px 0 0 0; opacity: 0.8; font-size: 14px; }
          </style>
        </head>
        <body>
          <div class="container">
            <h1>UI WebView</h1>
            <p>Blue - Created Second (Initially on Top)</p>
          </div>
        </body>
        </html>
      "#,
      )
      .with_bounds(Rect {
        position: LogicalPosition::new(200, 150).into(),
        size: LogicalSize::new(350, 250).into(),
      })
      .build_as_child(&window)
      .unwrap();

    println!("Z-Order Test Started");
    println!("===================");
    println!("Controls:");
    println!("  T - Toggle z-order");
    println!("  F - Bring content (red) to front");
    println!("  B - Send content (red) to back");
    println!("  Q/Escape - Quit");
    println!();
    println!("Initial state: UI (blue) on top");

    self.window = Some(window);
    self.content_webview = Some(content_webview);
    self.ui_webview = Some(ui_webview);
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
            PhysicalKey::Code(KeyCode::KeyT) => {
              // Toggle z-order
              if let (Some(content), Some(ui)) = (&self.content_webview, &self.ui_webview) {
                if self.content_on_top {
                  let _ = ui.bring_to_front();
                  self.content_on_top = false;
                  println!("UI (blue) brought to front");
                } else {
                  let _ = content.bring_to_front();
                  self.content_on_top = true;
                  println!("Content (red) brought to front");
                }
              }
            }
            PhysicalKey::Code(KeyCode::KeyF) => {
              // Bring content to front
              if let Some(content) = &self.content_webview {
                let _ = content.bring_to_front();
                self.content_on_top = true;
                println!("Content (red) brought to front");
              }
            }
            PhysicalKey::Code(KeyCode::KeyB) => {
              // Send content to back
              if let Some(content) = &self.content_webview {
                let _ = content.send_to_back();
                self.content_on_top = false;
                println!("Content (red) sent to back");
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
