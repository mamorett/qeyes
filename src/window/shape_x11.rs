use crate::render::build_xshape_bitmask;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use tiny_skia::Pixmap;
use winit::window::Window;
use x11rb::connection::Connection;
use x11rb::protocol::shape::{ConnectionExt as ShapeExt, SK, SO};
use x11rb::protocol::xproto::{
    ClipOrdering, ConnectionExt as XProtoExt, CreateGCAux, ImageFormat, Rectangle,
};
use x11rb::rust_connection::RustConnection;

pub struct X11ShapeManager {
    conn: RustConnection,
    screen_num: usize,
    compositor_active: bool,
}

impl X11ShapeManager {
    pub fn new(display_name: Option<&str>) -> Option<Self> {
        let (conn, screen_num) = x11rb::connect(display_name).ok()?;
        let compositor_active = Self::detect_compositor(&conn, screen_num);
        Some(Self {
            conn,
            screen_num,
            compositor_active,
        })
    }

    pub fn is_compositor_active(&self) -> bool {
        self.compositor_active
    }

    fn detect_compositor(conn: &RustConnection, screen_num: usize) -> bool {
        let atom_name = format!("_NET_WM_CM_S{screen_num}");
        let Ok(atom_cookie) = conn.intern_atom(false, atom_name.as_bytes()) else {
            return false;
        };
        let Ok(atom_reply) = atom_cookie.reply() else {
            return false;
        };
        if atom_reply.atom == x11rb::NONE {
            return false;
        }
        let Ok(sel_cookie) = conn.get_selection_owner(atom_reply.atom) else {
            return false;
        };
        let Ok(sel_reply) = sel_cookie.reply() else {
            return false;
        };
        sel_reply.owner != x11rb::NONE
    }

    fn extract_x11_window(window: &Window) -> Option<u32> {
        let handle = window.window_handle().ok()?;
        match handle.as_raw() {
            RawWindowHandle::Xlib(h) => Some(h.window as u32),
            RawWindowHandle::Xcb(h) => Some(h.window.get()),
            _ => None,
        }
    }

    /// Updates the X11 `Bounding` and `Input` shape regions for `window`.
    pub fn update_window_shapes(
        &self,
        window: &Window,
        pixmap: &Pixmap,
        shaped: bool,
        click_through: bool,
        force_xshape: bool,
    ) {
        let Some(win_id) = Self::extract_x11_window(window) else {
            return;
        };

        // 1. Update Input shape (controls click-through vs full-rect grip interaction)
        if click_through {
            let _ = self.conn.shape_rectangles(
                SO::SET,
                SK::INPUT,
                ClipOrdering::UNSORTED,
                win_id,
                0,
                0,
                &[],
            );
        } else {
            let full_rect = [Rectangle {
                x: 0,
                y: 0,
                width: pixmap.width() as u16,
                height: pixmap.height() as u16,
            }];
            let _ = self.conn.shape_rectangles(
                SO::SET,
                SK::INPUT,
                ClipOrdering::UNSORTED,
                win_id,
                0,
                0,
                &full_rect,
            );
        }

        // 2. Update Bounding shape when uncomposited, forced, or unshaped
        let use_xshape_bounding = shaped && (!self.compositor_active || force_xshape);
        if !use_xshape_bounding {
            let _ = self
                .conn
                .shape_mask(SO::SET, SK::BOUNDING, win_id, 0, 0, x11rb::NONE);
            let _ = self.conn.flush();
            return;
        }

        let w = pixmap.width() as u16;
        let h = pixmap.height() as u16;
        if w == 0 || h == 0 {
            return;
        }

        let bitmask = build_xshape_bitmask(pixmap, 96);
        let Ok(mask_pixmap) = self.conn.generate_id() else {
            return;
        };
        let Ok(gc) = self.conn.generate_id() else {
            return;
        };

        let root = self.conn.setup().roots[self.screen_num].root;
        if self.conn.create_pixmap(1, mask_pixmap, root, w, h).is_err() {
            return;
        }

        let _ = self.conn.create_gc(
            gc,
            mask_pixmap,
            &CreateGCAux::new().foreground(1).background(0),
        );
        let _ = self.conn.put_image(
            ImageFormat::XY_BITMAP,
            mask_pixmap,
            gc,
            w,
            h,
            0,
            0,
            0,
            1,
            &bitmask,
        );
        let _ = self
            .conn
            .shape_mask(SO::SET, SK::BOUNDING, win_id, 0, 0, mask_pixmap);
        let _ = self.conn.free_gc(gc);
        let _ = self.conn.free_pixmap(mask_pixmap);
        let _ = self.conn.flush();
    }
}
