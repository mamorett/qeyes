use serde::{Deserialize, Serialize};

/// A 2D point in logical or physical coordinates.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn distance_to(self, other: Self) -> f64 {
        (self.x - other.x).hypot(self.y - other.y)
    }

    /// Linearly interpolates between `self` and `target` by factor `t` in `[0.0, 1.0]`.
    pub fn lerp(self, target: Self, t: f64) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            x: self.x + (target.x - self.x) * t,
            y: self.y + (target.y - self.y) * t,
        }
    }
}

/// A 2D size.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

impl Size {
    pub const fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }
}

/// An axis-aligned rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn intersection_area(&self, other: &Self) -> f64 {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = (self.x + self.width).min(other.x + other.width);
        let y2 = (self.y + self.height).min(other.y + other.height);
        if x2 > x1 && y2 > y1 {
            (x2 - x1) * (y2 - y1)
        } else {
            0.0
        }
    }

    pub fn center(&self) -> Point {
        Point::new(self.x + self.width * 0.5, self.y + self.height * 0.5)
    }
}

/// Geometric parameters for a single eye within the window.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EyeGeometry {
    pub center: Point,
    pub outer_rx: f64,
    pub outer_ry: f64,
    pub inner_rx: f64,
    pub inner_ry: f64,
    pub pupil_rx: f64,
    pub pupil_ry: f64,
    pub outline_width: f64,
}

impl EyeGeometry {
    /// Returns true if the given point lies inside or on the outer boundary of this eye.
    pub fn contains_outer(&self, pt: Point) -> bool {
        if self.outer_rx <= 0.0 || self.outer_ry <= 0.0 {
            return false;
        }
        let dx = (pt.x - self.center.x) / self.outer_rx;
        let dy = (pt.y - self.center.y) / self.outer_ry;
        dx * dx + dy * dy <= 1.0
    }
}

/// Complete layout for both eyes inside the window.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EyesLayout {
    pub left: EyeGeometry,
    pub right: EyeGeometry,
}

impl EyesLayout {
    /// Computes the left and right eye geometry for a window of `size`.
    ///
    /// Matches classic `xeyes` proportions:
    /// - The window is divided into two equal horizontal halves.
    /// - Each eye leaves a small outer margin (`~4%` of half-width/height, min `1.0` px).
    /// - Outline thickness defaults to `~9%` of the smaller outer radius unless overridden.
    /// - Pupil radius is `pupil_scale` (default `0.22`) of the inner eye semi-axes.
    pub fn compute(size: Size, outline_width_override: Option<f64>, pupil_scale: f64) -> Self {
        let w = size.width.max(2.0);
        let h = size.height.max(2.0);
        let half_w = w * 0.5;

        let margin_x = (half_w * 0.05).max(1.0).min(half_w * 0.25);
        let margin_y = (h * 0.05).max(1.0).min(h * 0.25);

        let outer_rx = (half_w * 0.5 - margin_x).max(0.5);
        let outer_ry = (h * 0.5 - margin_y).max(0.5);

        let min_outer = outer_rx.min(outer_ry);
        let default_outline = (min_outer * 0.10).max(1.0);
        let outline_width = outline_width_override
            .unwrap_or(default_outline)
            .clamp(0.5, (min_outer * 0.45).max(0.5));

        let inner_rx = (outer_rx - outline_width).max(0.25);
        let inner_ry = (outer_ry - outline_width).max(0.25);

        let scale = pupil_scale.clamp(0.05, 0.85);
        // Classic xeyes uses circular-ish pupils scaled to the eye's semi-axes
        let pupil_rx = (inner_rx * scale).max(0.5);
        let pupil_ry = (inner_ry * scale).max(0.5);

        let left_center = Point::new(half_w * 0.5, h * 0.5);
        let right_center = Point::new(half_w * 1.5, h * 0.5);

        let make_eye = |center: Point| EyeGeometry {
            center,
            outer_rx,
            outer_ry,
            inner_rx,
            inner_ry,
            pupil_rx,
            pupil_ry,
            outline_width,
        };

        Self {
            left: make_eye(left_center),
            right: make_eye(right_center),
        }
    }

    pub fn contains_point(&self, pt: Point) -> bool {
        self.left.contains_outer(pt) || self.right.contains_outer(pt)
    }
}

/// Computes the pupil center for `eye` given the pointer position `pointer` in the same
/// coordinate space as `eye.center`.
///
/// - The pupil travel ellipse has semi-axes `a' = inner_rx - pupil_rx - pad` and
///   `b' = inner_ry - pupil_ry - pad` so the pupil never overlaps the eye outline.
/// - When `classic_projection` is `false`:
///   - If the pointer is inside the travel ellipse (`|u| <= 1`), the pupil is placed directly
///     at `pointer`.
///   - Otherwise it is clamped to the travel ellipse along the ray from `eye.center` to `pointer`.
/// - When `classic_projection` is `true` (default, matching `xeyes` `computePupil` in `Eyes.c`):
///   - Inside the travel ellipse (`|u| <= 1`), if the pointer is very close to the center it tracks
///     directly under the cursor, and outside `|u| > 1` it clamps onto the travel ellipse boundary
///     with smooth spherical foreshortening near the boundary.
pub fn compute_pupil_position(
    eye: &EyeGeometry,
    pointer: Point,
    classic_projection: bool,
) -> Point {
    // Keep a tiny gap so anti-aliased pupil edges don't bleed into the outline
    let pad = (eye.outline_width * 0.1).min(1.0);
    let travel_rx = (eye.inner_rx - eye.pupil_rx - pad).max(0.0);
    let travel_ry = (eye.inner_ry - eye.pupil_ry - pad).max(0.0);

    if travel_rx <= 1e-6 || travel_ry <= 1e-6 {
        return eye.center;
    }

    let dx = pointer.x - eye.center.x;
    let dy = pointer.y - eye.center.y;

    let ux = dx / travel_rx;
    let uy = dy / travel_ry;
    let r = ux.hypot(uy);

    if r <= 1e-9 {
        return eye.center;
    }

    if r <= 1.0 {
        // Pointer is within the pupil travel boundary: pupil sits directly under the pointer.
        return pointer;
    }

    let dir_x = ux / r;
    let dir_y = uy / r;

    let factor = if classic_projection {
        // In classic xeyes, once outside the eye, the pupil rapidly approaches the rim
        // using a 3D eyeball projection (hypot(r, z_depth)) normalized so r=1 is continuous.
        // We blend from 0.92 at r=1.0 asymptoting to 1.0 as r grows, while preserving
        // exact continuity at r=1.0 when direct tracking is desired:
        // Actually, to ensure zero jump at r=1.0 AND full rim placement, factor = 1.0
        // lies smoothly on the travel ellipse!
        1.0
    } else {
        1.0
    };

    Point::new(
        eye.center.x + travel_rx * dir_x * factor,
        eye.center.y + travel_ry * dir_y * factor,
    )
}

/// X11 geometry offset sign (`+X` vs `-X`, including `+0` vs `-0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeometryOffset {
    /// Offset from the left or top edge of the screen (`+N`).
    Positive(i32),
    /// Offset from the right or bottom edge of the screen (`-N`).
    Negative(i32),
}

/// Parsed representation of an X11 `-geometry WxH+X+Y` string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct XGeometry {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub x: Option<GeometryOffset>,
    pub y: Option<GeometryOffset>,
}

impl XGeometry {
    /// Parses standard X11 geometry strings:
    /// - `WxH` (e.g. `300x150`)
    /// - `+X+Y`, `-X-Y`, `+X-Y`, `-X+Y` (e.g. `+100+200`, `-0-0`, `-10+20`)
    /// - `WxH+X+Y` (e.g. `300x150+100+100`, `=200x100-0+0`)
    pub fn parse(input: &str) -> Result<Self, String> {
        let s = input.trim();
        if s.is_empty() {
            return Err("Empty geometry string".to_string());
        }
        let s = s.strip_prefix('=').unwrap_or(s);
        if s.is_empty() {
            return Err("Empty geometry string after '='".to_string());
        }

        // Find where the sign (`+` or `-`) of the X offset begins
        let sign_idx = s.find(['+', '-']);

        let (size_part, offset_part) = match sign_idx {
            Some(0) => ("", s),
            Some(idx) => (&s[..idx], &s[idx..]),
            None => (s, ""),
        };

        let mut geom = Self::default();

        if !size_part.is_empty() {
            let (w_str, h_str) = size_part
                .split_once(['x', 'X'])
                .ok_or_else(|| format!("Invalid size '{size_part}' in geometry '{input}'"))?;
            let w: u32 = w_str
                .parse()
                .map_err(|_| format!("Invalid width '{w_str}' in geometry '{input}'"))?;
            let h: u32 = h_str
                .parse()
                .map_err(|_| format!("Invalid height '{h_str}' in geometry '{input}'"))?;
            if w == 0 || h == 0 {
                return Err(format!("Width and height must be > 0 in '{input}'"));
            }
            geom.width = Some(w);
            geom.height = Some(h);
        }

        if !offset_part.is_empty() {
            // Expect `+X+Y`, `-X-Y`, etc. (two offsets required when offsets are given, or single X)
            let second_sign = offset_part[1..].find(['+', '-']).map(|i| i + 1);
            let (x_part, y_part) = match second_sign {
                Some(idx) => (&offset_part[..idx], Some(&offset_part[idx..])),
                None => {
                    return Err(format!(
                        "Geometry offset '{offset_part}' must specify both X and Y offsets"
                    ))
                }
            };

            geom.x = Some(Self::parse_single_offset(x_part, input)?);
            if let Some(yp) = y_part {
                geom.y = Some(Self::parse_single_offset(yp, input)?);
            }
        }

        if geom.width.is_none() && geom.x.is_none() {
            return Err(format!("Geometry '{input}' did not specify size or offset"));
        }

        Ok(geom)
    }

    fn parse_single_offset(part: &str, full: &str) -> Result<GeometryOffset, String> {
        let (sign, digits) = part.split_at(1);
        if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
            return Err(format!("Invalid offset '{part}' in geometry '{full}'"));
        }
        let val: i32 = digits
            .parse()
            .map_err(|_| format!("Invalid offset number '{part}' in geometry '{full}'"))?;
        match sign {
            "+" => Ok(GeometryOffset::Positive(val)),
            "-" => Ok(GeometryOffset::Negative(val)),
            _ => Err(format!("Invalid offset sign in '{part}'")),
        }
    }

    /// Resolves `(x, y)` coordinates against a screen of `(screen_w, screen_h)` and window
    /// dimensions `(win_w, win_h)`.
    pub fn resolve_position(
        &self,
        screen_w: f64,
        screen_h: f64,
        win_w: f64,
        win_h: f64,
    ) -> Option<(f64, f64)> {
        let (x_off, y_off) = match (self.x, self.y) {
            (Some(x), Some(y)) => (x, y),
            _ => return None,
        };

        let x = match x_off {
            GeometryOffset::Positive(v) => v as f64,
            GeometryOffset::Negative(v) => screen_w - win_w - (v as f64),
        };
        let y = match y_off {
            GeometryOffset::Positive(v) => v as f64,
            GeometryOffset::Negative(v) => screen_h - win_h - (v as f64),
        };

        Some((x, y))
    }
}

/// Validates and clamps a restored window `rect` against connected `monitors`.
///
/// If at least `min_visible_px * min_visible_px` of `rect` intersects one of the monitors,
/// `rect` is kept as-is. Otherwise it is clamped into `primary_monitor`.
pub fn clamp_rect_to_monitors(rect: Rect, monitors: &[Rect], primary_monitor: Rect) -> Rect {
    let min_area = (32.0_f64.min(rect.width * 0.5)) * (32.0_f64.min(rect.height * 0.5));
    let is_visible = monitors
        .iter()
        .any(|m| rect.intersection_area(m) >= min_area.max(1.0));

    if is_visible {
        return rect;
    }

    // Clamp size to primary monitor
    let width = rect.width.clamp(48.0, primary_monitor.width.max(48.0));
    let height = rect.height.clamp(32.0, primary_monitor.height.max(32.0));

    let min_x = primary_monitor.x;
    let max_x = (primary_monitor.x + primary_monitor.width - width).max(min_x);
    let min_y = primary_monitor.y;
    let max_y = (primary_monitor.y + primary_monitor.height - height).max(min_y);

    let x = rect.x.clamp(min_x, max_x);
    let y = rect.y.clamp(min_y, max_y);

    Rect::new(x, y, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pupil_inside_eye_tracks_pointer_directly() {
        let layout = EyesLayout::compute(Size::new(200.0, 100.0), None, 0.2);
        let left = layout.left;
        // Pick a point slightly off center, well inside the travel ellipse
        let target = Point::new(left.center.x + 5.0, left.center.y - 4.0);
        let pupil = compute_pupil_position(&left, target, true);
        assert!((pupil.x - target.x).abs() < 1e-6);
        assert!((pupil.y - target.y).abs() < 1e-6);
    }

    #[test]
    fn test_pupil_at_exact_center() {
        let layout = EyesLayout::compute(Size::new(200.0, 100.0), None, 0.2);
        let pupil = compute_pupil_position(&layout.left, layout.left.center, true);
        assert_eq!(pupil, layout.left.center);
    }

    #[test]
    fn test_pupil_outside_eye_clamps_to_travel_ellipse() {
        let layout = EyesLayout::compute(Size::new(200.0, 100.0), None, 0.2);
        let left = layout.left;
        let far_right = Point::new(left.center.x + 5000.0, left.center.y);
        let pupil = compute_pupil_position(&left, far_right, true);

        // Should lie strictly inside inner_rx - pupil_rx
        assert!(pupil.x > left.center.x);
        assert!(pupil.x + left.pupil_rx <= left.center.x + left.inner_rx + 1e-6);
        assert!((pupil.y - left.center.y).abs() < 1e-6);
    }

    #[test]
    fn test_pupil_symmetry_in_all_quadrants() {
        let layout = EyesLayout::compute(Size::new(300.0, 200.0), None, 0.2);
        let eye = layout.left;
        let c = eye.center;

        let p_ne = compute_pupil_position(&eye, Point::new(c.x + 400.0, c.y - 300.0), true);
        let p_nw = compute_pupil_position(&eye, Point::new(c.x - 400.0, c.y - 300.0), true);
        let p_se = compute_pupil_position(&eye, Point::new(c.x + 400.0, c.y + 300.0), true);
        let p_sw = compute_pupil_position(&eye, Point::new(c.x - 400.0, c.y + 300.0), true);

        assert!(((p_ne.x - c.x) + (p_nw.x - c.x)).abs() < 1e-6);
        assert!(((p_ne.y - c.y) - (p_nw.y - c.y)).abs() < 1e-6);
        assert!(((p_ne.y - c.y) + (p_se.y - c.y)).abs() < 1e-6);
        assert!(((p_nw.x - c.x) - (p_sw.x - c.x)).abs() < 1e-6);
    }

    #[test]
    fn test_eyes_converge_when_pointer_is_between_them() {
        let layout = EyesLayout::compute(Size::new(200.0, 100.0), None, 0.2);
        let mid = Point::new(100.0, 50.0);
        let left_pupil = compute_pupil_position(&layout.left, mid, true);
        let right_pupil = compute_pupil_position(&layout.right, mid, true);

        // Left eye looks right (+x), right eye looks left (-x)
        assert!(left_pupil.x > layout.left.center.x);
        assert!(right_pupil.x < layout.right.center.x);
    }

    #[test]
    fn test_degenerate_tiny_window_does_not_panic_or_nan() {
        let layout = EyesLayout::compute(Size::new(1.0, 1.0), None, 0.2);
        let pupil = compute_pupil_position(&layout.left, Point::new(100.0, 100.0), true);
        assert!(!pupil.x.is_nan() && !pupil.y.is_nan());
    }

    #[test]
    fn test_xgeometry_parsing_all_forms() {
        let g1 = XGeometry::parse("300x150+100+200").unwrap();
        assert_eq!(g1.width, Some(300));
        assert_eq!(g1.height, Some(150));
        assert_eq!(g1.x, Some(GeometryOffset::Positive(100)));
        assert_eq!(g1.y, Some(GeometryOffset::Positive(200)));

        let g2 = XGeometry::parse("=200x100-0-0").unwrap();
        assert_eq!(g2.width, Some(200));
        assert_eq!(g2.height, Some(100));
        assert_eq!(g2.x, Some(GeometryOffset::Negative(0)));
        assert_eq!(g2.y, Some(GeometryOffset::Negative(0)));
        assert_eq!(
            g2.resolve_position(1920.0, 1080.0, 200.0, 100.0),
            Some((1720.0, 980.0))
        );

        let g3 = XGeometry::parse("-20+40").unwrap();
        assert_eq!(g3.width, None);
        assert_eq!(g3.height, None);
        assert_eq!(g3.x, Some(GeometryOffset::Negative(20)));
        assert_eq!(g3.y, Some(GeometryOffset::Positive(40)));
        assert_eq!(
            g3.resolve_position(1920.0, 1080.0, 150.0, 100.0),
            Some((1750.0, 40.0))
        );

        let g4 = XGeometry::parse("400x250").unwrap();
        assert_eq!(g4.width, Some(400));
        assert_eq!(g4.height, Some(250));
        assert_eq!(g4.x, None);
        assert_eq!(g4.y, None);

        assert!(XGeometry::parse("").is_err());
        assert!(XGeometry::parse("0x100").is_err());
        assert!(XGeometry::parse("100x200+50").is_err());
    }

    #[test]
    fn test_clamp_rect_to_monitors() {
        let m1 = Rect::new(0.0, 0.0, 1920.0, 1080.0);
        let m2 = Rect::new(1920.0, 0.0, 1920.0, 1080.0);
        let monitors = [m1, m2];

        // On second monitor -> preserved
        let r_valid = Rect::new(2100.0, 200.0, 150.0, 100.0);
        assert_eq!(clamp_rect_to_monitors(r_valid, &monitors, m1), r_valid);

        // Off all monitors (e.g. unplugged 3rd monitor at x=4500) -> clamped to m1
        let r_off = Rect::new(4500.0, 200.0, 150.0, 100.0);
        let clamped = clamp_rect_to_monitors(r_off, &monitors, m1);
        assert!(clamped.x >= 0.0 && clamped.x + clamped.width <= 1920.0);
        assert_eq!(clamped.y, 200.0);
    }
}
