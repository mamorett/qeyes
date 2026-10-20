use crate::geometry::Point;
use crate::pointer::PointerSource;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::ConnectionExt;
use x11rb::rust_connection::RustConnection;

pub struct X11PointerSource {
    conn: RustConnection,
    root_window: u32,
}

impl X11PointerSource {
    pub fn new(display_name: Option<&str>) -> Option<Self> {
        let (conn, screen_num) = x11rb::connect(display_name).ok()?;
        let root_window = conn.setup().roots.get(screen_num)?.root;
        Some(Self { conn, root_window })
    }

    fn extract_x11_window(window: &Window) -> Option<u32> {
        let handle = window.window_handle().ok()?;
        match handle.as_raw() {
            RawWindowHandle::Xlib(h) => Some(h.window as u32),
            RawWindowHandle::Xcb(h) => Some(h.window.get()),
            _ => None,
        }
    }
}

impl PointerSource for X11PointerSource {
    fn cursor_in_window(&mut self, window: &Window) -> Option<Point> {
        if let Some(win_id) = Self::extract_x11_window(window) {
            if let Ok(cookie) = self.conn.query_pointer(win_id) {
                if let Ok(reply) = cookie.reply() {
                    return Some(Point::new(reply.win_x as f64, reply.win_y as f64));
                }
            }
        }

        // Fallback: query root window and subtract outer/inner position from winit
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
        let reply = self
            .conn
            .query_pointer(self.root_window)
            .ok()?
            .reply()
            .ok()?;
        Some(Point::new(reply.root_x as f64, reply.root_y as f64))
    }
}
