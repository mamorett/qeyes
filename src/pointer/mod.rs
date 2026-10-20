#[cfg(target_os = "macos")]
pub mod macos;
pub mod wayland_stub;
#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "linux")]
pub mod x11;

use crate::geometry::Point;
use winit::window::Window;

/// Trait for querying the global mouse pointer position mapped into the window's physical
/// pixel coordinate space.
pub trait PointerSource {
    /// Returns the pointer position in physical pixels relative to the top-left corner of
    /// `window`'s client area. Coordinates may be negative or exceed the window dimensions
    /// when the cursor is outside the window.
    fn cursor_in_window(&mut self, window: &Window) -> Option<Point>;

    /// Returns the raw global screen pointer position in physical pixels (used by `--dump-state`).
    fn global_position(&mut self) -> Option<Point>;
}

/// Creates the platform-appropriate `PointerSource`.
pub fn create_pointer_source(display_name: Option<&str>) -> Box<dyn PointerSource> {
    #[cfg(target_os = "linux")]
    {
        if let Some(src) = x11::X11PointerSource::new(display_name) {
            return Box::new(src);
        }
        Box::new(wayland_stub::WaylandFallbackSource::new())
    }

    #[cfg(target_os = "windows")]
    {
        let _ = display_name;
        Box::new(windows::WindowsPointerSource::new())
    }

    #[cfg(target_os = "macos")]
    {
        let _ = display_name;
        Box::new(macos::MacOsPointerSource::new())
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        let _ = display_name;
        Box::new(wayland_stub::WaylandFallbackSource::new())
    }
}
