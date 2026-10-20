use crate::geometry::Point;
use crate::pointer::PointerSource;
use winit::window::Window;

/// Fallback pointer source used when no global display query is available.
pub struct WaylandFallbackSource;

impl WaylandFallbackSource {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WaylandFallbackSource {
    fn default() -> Self {
        Self::new()
    }
}

impl PointerSource for WaylandFallbackSource {
    fn cursor_in_window(&mut self, _window: &Window) -> Option<Point> {
        None
    }

    fn global_position(&mut self) -> Option<Point> {
        None
    }
}
