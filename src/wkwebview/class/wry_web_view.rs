// Copyright 2020-2024 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{
  cell::Cell,
  collections::HashMap,
  sync::{atomic::AtomicU64, Mutex},
};

#[cfg(target_os = "macos")]
use objc2::runtime::ProtocolObject;
use objc2::{define_class, rc::Retained, runtime::Bool, DeclaredClass};
#[cfg(target_os = "macos")]
use objc2_app_kit::{NSDraggingDestination, NSEvent, NSView};
use objc2_foundation::{NSObjectProtocol, NSUUID};

#[cfg(target_os = "macos")]
use objc2_core_foundation::{CGPoint, CGRect};

#[cfg(target_os = "ios")]
use crate::wkwebview::ios::WKWebView::WKWebView;
use crate::HitTestMode;
#[cfg(target_os = "macos")]
use crate::{
  wkwebview::{drag_drop, synthetic_mouse_events},
  DragDropEvent,
};
#[cfg(target_os = "ios")]
use objc2_ui_kit::UIEvent as NSEvent;
#[cfg(target_os = "macos")]
use objc2_web_kit::WKWebView;

pub struct WryWebViewIvars {
  pub(crate) is_child: bool,
  #[cfg(target_os = "macos")]
  pub(crate) drag_drop_handler: Box<dyn Fn(DragDropEvent) -> bool>,
  #[cfg(target_os = "macos")]
  pub(crate) accept_first_mouse: objc2::runtime::Bool,
  #[cfg(target_os = "ios")]
  pub(crate) input_accessory_view_builder: Option<Box<crate::InputAccessoryViewBuilder>>,
  pub(crate) custom_protocol_task_ids: Mutex<HashMap<usize, Retained<NSUUID>>>,

  /// Current hit-test mode
  pub(crate) hit_test_mode: Cell<HitTestMode>,

  /// Interactive regions (only used in RegionBased mode)
  /// Stored as (id, CGRect in webview-local coordinates with top-left origin)
  #[cfg(target_os = "macos")]
  pub(crate) hit_regions: Mutex<Vec<(u64, CGRect)>>,

  /// Counter for generating unique region IDs
  pub(crate) hit_region_counter: AtomicU64,
}

define_class!(
  #[unsafe(super(WKWebView))]
  #[name = "WryWebView"]
  #[ivars = WryWebViewIvars]
  pub struct WryWebView;

  /// Overridden NSView methods.
  impl WryWebView {
    #[unsafe(method(performKeyEquivalent:))]
    fn perform_key_equivalent(&self, event: &NSEvent) -> Bool {
      // This is a temporary workaround for https://github.com/tauri-apps/tauri/issues/9426
      // FIXME: When the webview is a child webview, performKeyEquivalent always return YES
      // and stop propagating the event to the window, hence the menu shortcut won't be
      // triggered. However, overriding this method also means the cmd+key event won't be
      // handled in webview, which means the key cannot be listened by JavaScript.
      if self.ivars().is_child {
        Bool::NO
      } else {
        unsafe { objc2::msg_send![super(self), performKeyEquivalent: event] }
      }
    }

    #[cfg(target_os = "macos")]
    #[unsafe(method(acceptsFirstMouse:))]
    fn accept_first_mouse(&self, _event: &NSEvent) -> Bool {
      self.ivars().accept_first_mouse
    }

    /// Override hitTest: to support region-based and passthrough hit testing.
    ///
    /// hitTest: receives a point in the superview's coordinate system and returns
    /// the deepest subview that should receive the event, or nil to pass through.
    #[cfg(target_os = "macos")]
    #[unsafe(method_id(hitTest:))]
    fn hit_test(&self, point: objc2_core_foundation::CGPoint) -> Option<Retained<NSView>> {
      hit_test_impl(self, point)
    }

    #[cfg(target_os = "ios")]
    #[unsafe(method_id(inputAccessoryView))]
    fn input_accessory_view(&self) -> Option<Retained<objc2_ui_kit::UIView>> {
      if let Some(builder) = &self.ivars().input_accessory_view_builder {
        builder(self)
      } else {
        unsafe { objc2::msg_send![super(self), inputAccessoryView] }
      }
    }
  }
  unsafe impl NSObjectProtocol for WryWebView {}

  // Drag & Drop
  #[cfg(target_os = "macos")]
  unsafe impl NSDraggingDestination for WryWebView {
    #[unsafe(method(draggingEntered:))]
    fn dragging_entered(
      &self,
      drag_info: &ProtocolObject<dyn objc2_app_kit::NSDraggingInfo>,
    ) -> objc2_app_kit::NSDragOperation {
      drag_drop::dragging_entered(self, drag_info)
    }

    #[unsafe(method(draggingUpdated:))]
    fn dragging_updated(
      &self,
      drag_info: &ProtocolObject<dyn objc2_app_kit::NSDraggingInfo>,
    ) -> objc2_app_kit::NSDragOperation {
      drag_drop::dragging_updated(self, drag_info)
    }

    #[unsafe(method(performDragOperation:))]
    fn perform_drag_operation(
      &self,
      drag_info: &ProtocolObject<dyn objc2_app_kit::NSDraggingInfo>,
    ) -> Bool {
      drag_drop::perform_drag_operation(self, drag_info)
    }

    #[unsafe(method(draggingExited:))]
    fn dragging_exited(&self, drag_info: &ProtocolObject<dyn objc2_app_kit::NSDraggingInfo>) {
      drag_drop::dragging_exited(self, drag_info)
    }
  }

  // Synthetic mouse events
  #[cfg(target_os = "macos")]
  impl WryWebView {
    #[unsafe(method(otherMouseDown:))]
    fn other_mouse_down(&self, event: &NSEvent) {
      synthetic_mouse_events::other_mouse_down(self, event)
    }

    #[unsafe(method(otherMouseUp:))]
    fn other_mouse_up(&self, event: &NSEvent) {
      synthetic_mouse_events::other_mouse_up(self, event)
    }
  }
);

/// Implementation of hit-test logic extracted to avoid macro issues with early returns.
#[cfg(target_os = "macos")]
fn hit_test_impl(
  view: &WryWebView,
  point: objc2_core_foundation::CGPoint,
) -> Option<Retained<NSView>> {
  use HitTestMode::*;

  let mode = view.ivars().hit_test_mode.get();

  // PassThrough mode - never capture events
  if matches!(mode, PassThrough) {
    return None;
  }

  // Normal mode - default behavior
  if matches!(mode, Normal) {
    return unsafe { objc2::msg_send![super(view), hitTest: point] };
  }

  // RegionBased mode - check if point is in an interactive region
  // Convert point to local coordinates (hitTest: receives superview coordinates)
  let local_point: CGPoint =
    unsafe { objc2::msg_send![view, convertPoint: point, fromView: std::ptr::null::<NSView>()] };

  // Get the frame to check bounds and convert Y coordinate
  let frame: CGRect = unsafe { objc2::msg_send![view, frame] };

  // Check if view is flipped (WKWebView typically IS flipped)
  let is_flipped: bool = unsafe { objc2::msg_send![view, isFlipped] };

  // Check if point is within our frame bounds
  let in_bounds = local_point.x >= 0.0
    && local_point.y >= 0.0
    && local_point.x <= frame.size.width
    && local_point.y <= frame.size.height;

  if !in_bounds {
    return None;
  }

  // If view is flipped, local_point.y is already in web coordinates (top-left origin)
  // If not flipped, we need to convert from macOS coords (bottom-left origin)
  let web_y = if is_flipped {
    local_point.y
  } else {
    frame.size.height - local_point.y
  };

  // Check if point is in any active region
  let in_active_region = {
    let regions = view.ivars().hit_regions.lock().unwrap();
    regions.iter().any(|(_, rect)| {
      local_point.x >= rect.origin.x
        && local_point.x <= rect.origin.x + rect.size.width
        && web_y >= rect.origin.y
        && web_y <= rect.origin.y + rect.size.height
    })
  };

  if in_active_region {
    // Point is in an interactive region - handle normally
    unsafe { objc2::msg_send![super(view), hitTest: point] }
  } else {
    // Point is outside interactive regions - pass through
    None
  }
}

// Custom Protocol Task Checker
impl WryWebView {
  pub(crate) fn add_custom_task_key(&self, task_id: usize) -> Retained<NSUUID> {
    let task_uuid = NSUUID::new();
    self
      .ivars()
      .custom_protocol_task_ids
      .lock()
      .unwrap()
      .insert(task_id, task_uuid.clone());
    task_uuid
  }
  pub(crate) fn remove_custom_task_key(&self, task_id: usize) {
    self
      .ivars()
      .custom_protocol_task_ids
      .lock()
      .unwrap()
      .remove(&task_id);
  }
  pub(crate) fn get_custom_task_uuid(&self, task_id: usize) -> Option<Retained<NSUUID>> {
    self
      .ivars()
      .custom_protocol_task_ids
      .lock()
      .unwrap()
      .get(&task_id)
      .cloned()
  }
}
