use crate::geometry::Point;
use crate::pointer::PointerSource;
use objc2_app_kit::{NSEvent, NSScreen, NSView};
use objc2_foundation::{MainThreadMarker, NSPoint};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

pub struct MacOsPointerSource;

impl MacOsPointerSource {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MacOsPointerSource {
    fn default() -> Self {
        Self::new()
    }
}

impl PointerSource for MacOsPointerSource {
    fn cursor_in_window(&mut self, window: &Window) -> Option<Point> {
        let scale = window.scale_factor();
        let handle = window.window_handle().ok()?;
        if let RawWindowHandle::AppKit(appkit) = handle.as_raw() {
            unsafe {
                let ns_view_ptr = appkit.ns_view.as_ptr() as *const NSView;
                let ns_view: &NSView = &*ns_view_ptr;
                if let Some(ns_window) = ns_view.window() {
                    let screen_pt: NSPoint = NSEvent::mouseLocation();
                    let win_pt: NSPoint = ns_window.convertPointFromScreen(screen_pt);
                    let view_frame = ns_view.frame();
                    // Cocoa window coordinates have origin at bottom-left; convert to top-left physical pixels
                    let local_x = win_pt.x * scale;
                    let local_y = (view_frame.size.height - win_pt.y) * scale;
                    return Some(Point::new(local_x, local_y));
                }
            }
        }

        let global = self.global_position()?;
        let win_pos = window
            .inner_position()
            .or_else(|_| window.outer_position())
            .ok()?;
        Some(Point::new(
            global.x - win_pos.x as f64,
            global.y - win_pos.y as f64,
        ))
    }

    fn global_position(&mut self) -> Option<Point> {
        unsafe {
            let screen_pt: NSPoint = NSEvent::mouseLocation();
            let mtm = MainThreadMarker::new_unchecked();
            let screens = NSScreen::screens(mtm);
            let primary_h = screens
                .first()
                .map(|s| s.frame().size.height)
                .unwrap_or(1080.0);
            Some(Point::new(screen_pt.x, primary_h - screen_pt.y))
        }
    }
}
