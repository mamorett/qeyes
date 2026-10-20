use crate::geometry::Point;
use crate::pointer::PointerSource;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows_sys::Win32::Foundation::{HWND, POINT};
use windows_sys::Win32::Graphics::Gdi::ScreenToClient;
use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
use winit::window::Window;

pub struct WindowsPointerSource;

impl WindowsPointerSource {
    pub fn new() -> Self {
        Self
    }

    fn extract_hwnd(window: &Window) -> Option<HWND> {
        let handle = window.window_handle().ok()?;
        match handle.as_raw() {
            RawWindowHandle::Win32(h) => Some(h.hwnd.get() as HWND),
            _ => None,
        }
    }
}

impl Default for WindowsPointerSource {
    fn default() -> Self {
        Self::new()
    }
}

impl PointerSource for WindowsPointerSource {
    fn cursor_in_window(&mut self, window: &Window) -> Option<Point> {
        unsafe {
            let mut pt = POINT { x: 0, y: 0 };
            if GetCursorPos(&mut pt) == 0 {
                return None;
            }
            if let Some(hwnd) = Self::extract_hwnd(window) {
                if ScreenToClient(hwnd, &mut pt) != 0 {
                    return Some(Point::new(pt.x as f64, pt.y as f64));
                }
            }
            let win_pos = window
                .inner_position()
                .or_else(|_| window.outer_position())
                .ok()?;
            Some(Point::new(
                (pt.x - win_pos.x) as f64,
                (pt.y - win_pos.y) as f64,
            ))
        }
    }

    fn global_position(&mut self) -> Option<Point> {
        unsafe {
            let mut pt = POINT { x: 0, y: 0 };
            if GetCursorPos(&mut pt) != 0 {
                Some(Point::new(pt.x as f64, pt.y as f64))
            } else {
                None
            }
        }
    }
}
