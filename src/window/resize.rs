use crate::geometry::{Point, Size};
use winit::window::{CursorIcon, ResizeDirection};

pub const MIN_LOGICAL_WIDTH: f64 = 48.0;
pub const MIN_LOGICAL_HEIGHT: f64 = 32.0;
pub const EDGE_GRIP_LOGICAL_PX: f64 = 8.0;
pub const CORNER_GRIP_LOGICAL_PX: f64 = 14.0;

/// Hit-tests whether a point `pt` (in physical pixels) lies within the outer resize border/corners
/// of a borderless window of size `win_size` (in physical pixels) with HiDPI `scale_factor`.
pub fn hit_test_resize_grip(
    pt: Point,
    win_size: Size,
    scale_factor: f64,
) -> Option<ResizeDirection> {
    let s = scale_factor.max(1.0);
    let edge = EDGE_GRIP_LOGICAL_PX * s;
    let corner = CORNER_GRIP_LOGICAL_PX * s;

    let w = win_size.width;
    let h = win_size.height;

    if pt.x < 0.0 || pt.y < 0.0 || pt.x > w || pt.y > h {
        return None;
    }

    let on_left = pt.x <= edge;
    let on_right = pt.x >= w - edge;
    let on_top = pt.y <= edge;
    let on_bottom = pt.y >= h - edge;

    if on_left || on_right || on_top || on_bottom {
        let near_left = pt.x <= corner;
        let near_right = pt.x >= w - corner;
        let near_top = pt.y <= corner;
        let near_bottom = pt.y >= h - corner;

        return match (near_left, near_right, near_top, near_bottom) {
            (true, _, true, _) => Some(ResizeDirection::NorthWest),
            (_, true, true, _) => Some(ResizeDirection::NorthEast),
            (true, _, _, true) => Some(ResizeDirection::SouthWest),
            (_, true, _, true) => Some(ResizeDirection::SouthEast),
            _ if on_top => Some(ResizeDirection::North),
            _ if on_bottom => Some(ResizeDirection::South),
            _ if on_left => Some(ResizeDirection::West),
            _ if on_right => Some(ResizeDirection::East),
            _ => None,
        };
    }

    // Also hit-test the outer curved outline rim of the left/right eyes for shaped windows
    let rx = (w * 0.25).max(1.0);
    let ry = (h * 0.50).max(1.0);
    let left_cx = w * 0.25;
    let right_cx = w * 0.75;
    let cy = h * 0.50;

    if pt.x <= left_cx || pt.x >= right_cx {
        let cx = if pt.x <= w * 0.5 { left_cx } else { right_cx };
        let nx = (pt.x - cx) / rx;
        let ny = (pt.y - cy) / ry;
        let r_out = (nx * nx + ny * ny).sqrt();

        let rx_in = (rx - edge).max(1.0);
        let ry_in = (ry - edge).max(1.0);
        let nx_in = (pt.x - cx) / rx_in;
        let ny_in = (pt.y - cy) / ry_in;
        let r_in = (nx_in * nx_in + ny_in * ny_in).sqrt();

        if r_out <= 1.05 && r_in >= 1.0 {
            if ny < -0.35 && nx < -0.35 {
                return Some(ResizeDirection::NorthWest);
            } else if ny > 0.35 && nx < -0.35 {
                return Some(ResizeDirection::SouthWest);
            } else if ny < -0.35 && nx > 0.35 {
                return Some(ResizeDirection::NorthEast);
            } else if ny > 0.35 && nx > 0.35 {
                return Some(ResizeDirection::SouthEast);
            } else if ny < -0.75 {
                return Some(ResizeDirection::North);
            } else if ny > 0.75 {
                return Some(ResizeDirection::South);
            } else if nx < 0.0 {
                return Some(ResizeDirection::West);
            } else {
                return Some(ResizeDirection::East);
            }
        }
    }

    None
}

/// Maps a `ResizeDirection` to the appropriate `winit::window::CursorIcon`.
pub fn direction_to_cursor_icon(dir: Option<ResizeDirection>) -> CursorIcon {
    match dir {
        Some(ResizeDirection::North) => CursorIcon::NResize,
        Some(ResizeDirection::South) => CursorIcon::SResize,
        Some(ResizeDirection::East) => CursorIcon::EResize,
        Some(ResizeDirection::West) => CursorIcon::WResize,
        Some(ResizeDirection::NorthWest) => CursorIcon::NwResize,
        Some(ResizeDirection::NorthEast) => CursorIcon::NeResize,
        Some(ResizeDirection::SouthWest) => CursorIcon::SwResize,
        Some(ResizeDirection::SouthEast) => CursorIcon::SeResize,
        None => CursorIcon::Default,
    }
}

/// Scales a logical window size by `factor` (e.g. `1.1` or `0.9`), clamping to constraints.
pub fn scale_logical_size(
    current: Size,
    factor: f64,
    max_size: Size,
    lock_aspect: bool,
) -> (u32, u32) {
    let max_w = max_size.width.max(MIN_LOGICAL_WIDTH);
    let max_h = max_size.height.max(MIN_LOGICAL_HEIGHT);

    let mut new_w = (current.width * factor).clamp(MIN_LOGICAL_WIDTH, max_w);
    let mut new_h = (current.height * factor).clamp(MIN_LOGICAL_HEIGHT, max_h);

    if lock_aspect && current.height > 0.0 {
        let ratio = current.width / current.height;
        new_h = (new_w / ratio).clamp(MIN_LOGICAL_HEIGHT, max_h);
        new_w = (new_h * ratio).clamp(MIN_LOGICAL_WIDTH, max_w);
    }

    (new_w.round() as u32, new_h.round() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resize_grip_hit_testing() {
        let size = Size::new(200.0, 100.0);
        assert_eq!(
            hit_test_resize_grip(Point::new(2.0, 2.0), size, 1.0),
            Some(ResizeDirection::NorthWest)
        );
        assert_eq!(
            hit_test_resize_grip(Point::new(198.0, 2.0), size, 1.0),
            Some(ResizeDirection::NorthEast)
        );
        assert_eq!(
            hit_test_resize_grip(Point::new(2.0, 98.0), size, 1.0),
            Some(ResizeDirection::SouthWest)
        );
        assert_eq!(
            hit_test_resize_grip(Point::new(198.0, 98.0), size, 1.0),
            Some(ResizeDirection::SouthEast)
        );
        assert_eq!(
            hit_test_resize_grip(Point::new(100.0, 3.0), size, 1.0),
            Some(ResizeDirection::North)
        );
        assert_eq!(
            hit_test_resize_grip(Point::new(100.0, 50.0), size, 1.0),
            None
        );
    }

    #[test]
    fn test_scale_logical_size() {
        let s = Size::new(150.0, 100.0);
        let max = Size::new(1920.0, 1080.0);
        assert_eq!(scale_logical_size(s, 2.0, max, true), (300, 200));
        assert_eq!(scale_logical_size(s, 0.01, max, false), (48, 32));
    }
}
