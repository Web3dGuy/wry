// Copyright 2020-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! Snapshot Timing Test Example
//!
//! This example measures the latency of WKWebView's takeSnapshot API,
//! which is relevant for offscreen rendering approaches to compositing.
//!
//! Key metrics to observe:
//! - Time to capture snapshot (should ideally be < 16.67ms for 60fps)
//! - Image data size (affects GPU upload time)
//! - Consistency across multiple samples

use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tao::{
  dpi::LogicalSize,
  event::{Event, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  window::WindowBuilder,
};
use wry::WebViewBuilder;

#[cfg(target_os = "macos")]
use wry::WebViewExtMacOS;

#[derive(Debug)]
enum Command {
  Snapshot,
  Stats,
  Clear,
}

const TEST_HTML: &str = r#"
<!DOCTYPE html>
<html>
<head>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
      min-height: 100vh;
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      color: white;
    }
    h1 { font-size: 48px; margin-bottom: 20px; }
    .box {
      width: 200px;
      height: 200px;
      background: rgba(255,255,255,0.2);
      border-radius: 20px;
      animation: rotate 3s linear infinite;
    }
    @keyframes rotate {
      from { transform: rotate(0deg); }
      to { transform: rotate(360deg); }
    }
    .stats {
      position: fixed;
      top: 20px;
      right: 20px;
      background: rgba(0,0,0,0.7);
      padding: 15px;
      border-radius: 8px;
      font-family: monospace;
      font-size: 14px;
    }
    .instruction {
      position: fixed;
      bottom: 20px;
      left: 50%;
      transform: translateX(-50%);
      background: rgba(0,0,0,0.7);
      padding: 10px 20px;
      border-radius: 8px;
    }
  </style>
</head>
<body>
  <h1>Snapshot Timing Test</h1>
  <div class="box"></div>
  <div class="stats" id="stats">
    Press T to run snapshot test<br>
    Press S to show statistics<br>
    Press C to clear statistics
  </div>
  <div class="instruction">
    Testing WKWebView.takeSnapshot() latency
  </div>
  <script>
    let frameCount = 0;
    let lastTime = performance.now();
    function updateFPS() {
      frameCount++;
      const now = performance.now();
      if (now - lastTime >= 1000) {
        frameCount = 0;
        lastTime = now;
      }
      requestAnimationFrame(updateFPS);
    }
    updateFPS();

    // Handle keyboard via IPC since webview captures focus
    document.addEventListener('keydown', (e) => {
      const key = e.key.toLowerCase();
      if (key === 't') {
        window.ipc.postMessage('snapshot');
      } else if (key === 's') {
        window.ipc.postMessage('stats');
      } else if (key === 'c') {
        window.ipc.postMessage('clear');
      }
    });
  </script>
</body>
</html>
"#;

fn main() -> wry::Result<()> {
  let event_loop = EventLoop::new();
  let window = WindowBuilder::new()
    .with_title("Snapshot Timing Test - WRY")
    .with_inner_size(LogicalSize::new(800.0, 600.0))
    .build(&event_loop)
    .unwrap();

  // Channel for IPC -> event loop communication
  let (tx, rx): (Sender<Command>, Receiver<Command>) = channel();

  // Timing statistics
  let snapshot_times: Arc<Mutex<Vec<Duration>>> = Arc::new(Mutex::new(Vec::new()));

  let webview = WebViewBuilder::new()
    .with_html(TEST_HTML)
    .with_ipc_handler(move |req| {
      let body = req.body();
      let cmd = match body.as_str() {
        "snapshot" => Some(Command::Snapshot),
        "stats" => Some(Command::Stats),
        "clear" => Some(Command::Clear),
        _ => None,
      };
      if let Some(cmd) = cmd {
        let _ = tx.send(cmd);
      }
    })
    .build(&window)?;

  println!("=== Snapshot Timing Test ===");
  println!("Press 'T' to run a single snapshot test");
  println!("Press 'S' to show statistics");
  println!("Press 'C' to clear statistics");
  println!();

  event_loop.run(move |event, _, control_flow| {
    *control_flow = ControlFlow::Poll; // Use Poll to check channel

    // Check for IPC commands
    while let Ok(cmd) = rx.try_recv() {
      match cmd {
        Command::Snapshot => {
          #[cfg(target_os = "macos")]
          {
            println!("Running single snapshot test...");
            let times = snapshot_times.clone();
            webview.take_snapshot(move |duration, image_data| {
              let size_kb = image_data.map(|d| d.len() / 1024).unwrap_or(0);
              println!("Snapshot completed: {:?} ({} KB TIFF)", duration, size_kb);
              times.lock().unwrap().push(duration);
            });
          }
          #[cfg(not(target_os = "macos"))]
          {
            println!("Snapshot API only available on macOS");
          }
        }
        Command::Stats => {
          let times = snapshot_times.lock().unwrap();
          if times.is_empty() {
            println!("No samples collected yet");
          } else {
            let total: Duration = times.iter().sum();
            let avg = total / times.len() as u32;
            let min = times.iter().min().unwrap();
            let max = times.iter().max().unwrap();
            println!();
            println!("=== Statistics ({} samples) ===", times.len());
            println!("  Average: {:?}", avg);
            println!("  Min:     {:?}", min);
            println!("  Max:     {:?}", max);
            println!(
              "  Theoretical FPS: {:.1}",
              1000.0 / avg.as_millis() as f64
            );
            println!(
              "  60fps capable: {}",
              if avg.as_millis() < 17 { "YES" } else { "NO" }
            );
          }
        }
        Command::Clear => {
          snapshot_times.lock().unwrap().clear();
          println!("Statistics cleared");
        }
      }
    }

    match event {
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
