pub mod theme;

use crate::config::Settings;
use crate::geometry::{EyeGeometry, EyesLayout, Point, Size};
use crate::render::theme::Color;
use crate::ui::menu::{glyph_5x7, ContextMenuState};
use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Rect as SkiaRect, Transform};

/// Renders a complete frame of `qeyes` into a `tiny_skia::Pixmap`.
///
/// - `width_px` and `height_px`: physical pixel dimensions of the surface.
/// - `scale_factor`: HiDPI scale factor (used to scale explicit logical border widths and menu UI).
/// - `left_pupil` and `right_pupil`: pupil centers in physical pixel coordinates.
/// - `blink_openness`: `1.0` = eyes fully open, `0.0` = eyelids completely closed.
/// - `menu`: optional open context menu overlay.
#[allow(clippy::too_many_arguments)]
pub fn render_eyes_frame(
    settings: &Settings,
    width_px: u32,
    height_px: u32,
    scale_factor: f64,
    left_pupil: Point,
    right_pupil: Point,
    blink_openness: f64,
    menu: Option<&ContextMenuState>,
) -> Pixmap {
    let w = width_px.max(1);
    let h = height_px.max(1);
    let mut pixmap = Pixmap::new(w, h).expect("Failed to allocate tiny-skia Pixmap");

    let colors = settings.resolved_colors();
    let antialias = settings.appearance.antialias;

    // 1. Background outside eyes
    if !settings.appearance.shaped {
        pixmap.fill(colors.background.to_tiny_skia());
    } else {
        pixmap.fill(tiny_skia::Color::TRANSPARENT);
    }

    let outline_override = settings
        .appearance
        .outline_width
        .map(|bw| bw * scale_factor.max(1.0));
    let layout = EyesLayout::compute(
        Size::new(w as f64, h as f64),
        outline_override,
        settings.appearance.pupil_scale,
    );

    // 2. Draw left and right eyes
    draw_single_eye(
        &mut pixmap,
        &layout.left,
        left_pupil,
        colors.outline,
        colors.background,
        colors.center,
        antialias,
        blink_openness,
    );
    draw_single_eye(
        &mut pixmap,
        &layout.right,
        right_pupil,
        colors.outline,
        colors.background,
        colors.center,
        antialias,
        blink_openness,
    );

    // 3. Draw optional right-click context menu overlay
    if let Some(menu_state) = menu {
        if menu_state.visible {
            draw_context_menu(&mut pixmap, settings, menu_state, scale_factor);
        }
    }

    pixmap
}

#[allow(clippy::too_many_arguments)]
fn draw_single_eye(
    pixmap: &mut Pixmap,
    eye: &EyeGeometry,
    pupil: Point,
    outline_color: Color,
    sclera_color: Color,
    pupil_color: Color,
    antialias: bool,
    blink_openness: f64,
) {
    // 1. Outer ellipse (outline color)
    fill_ellipse(
        pixmap,
        eye.center,
        eye.outer_rx,
        eye.outer_ry,
        outline_color,
        antialias,
    );

    let openness = blink_openness.clamp(0.0, 1.0);
    if openness <= 0.02 {
        // Eye is fully blinked shut: draw a thin horizontal crease in sclera/background color
        let crease_ry = (eye.outline_width * 0.35).max(0.8);
        fill_ellipse(
            pixmap,
            eye.center,
            eye.inner_rx,
            crease_ry,
            sclera_color,
            antialias,
        );
        return;
    }

    // 2. Sclera (inner ellipse, vertically scaled by blink openness when blinking)
    let active_inner_ry = (eye.inner_ry * openness).max(0.5);
    fill_ellipse(
        pixmap,
        eye.center,
        eye.inner_rx,
        active_inner_ry,
        sclera_color,
        antialias,
    );

    // 3. Pupil (clamped vertically if eyelid is partially closing during a blink)
    let active_pupil_ry = eye.pupil_ry.min(active_inner_ry * 0.9);
    let max_dy = (active_inner_ry - active_pupil_ry).max(0.0);
    let clamped_pupil_y = pupil.y.clamp(eye.center.y - max_dy, eye.center.y + max_dy);

    fill_ellipse(
        pixmap,
        Point::new(pupil.x, clamped_pupil_y),
        eye.pupil_rx,
        active_pupil_ry,
        pupil_color,
        antialias,
    );
}

/// Fills an axis-aligned ellipse centered at `center` with semi-axes `(rx, ry)`.
///
/// Uses a cubic Bézier approximation (`KAPPA = 0.552284749831`) via `tiny-skia`.
fn fill_ellipse(
    pixmap: &mut Pixmap,
    center: Point,
    rx: f64,
    ry: f64,
    color: Color,
    antialias: bool,
) {
    if rx <= 0.0 || ry <= 0.0 {
        return;
    }

    const KAPPA: f32 = 0.552_284_8;
    let cx = center.x as f32;
    let cy = center.y as f32;
    let rx = rx as f32;
    let ry = ry as f32;
    let ox = rx * KAPPA;
    let oy = ry * KAPPA;

    let mut pb = PathBuilder::new();
    pb.move_to(cx - rx, cy);
    pb.cubic_to(cx - rx, cy - oy, cx - ox, cy - ry, cx, cy - ry);
    pb.cubic_to(cx + ox, cy - ry, cx + rx, cy - oy, cx + rx, cy);
    pb.cubic_to(cx + rx, cy + oy, cx + ox, cy + ry, cx, cy + ry);
    pb.cubic_to(cx - ox, cy + ry, cx - rx, cy + oy, cx - rx, cy);
    pb.close();

    if let Some(path) = pb.finish() {
        let mut paint = Paint::default();
        paint.set_color(color.to_tiny_skia());
        paint.anti_alias = antialias;
        pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
}

fn draw_context_menu(
    pixmap: &mut Pixmap,
    settings: &Settings,
    menu_state: &ContextMenuState,
    scale_factor: f64,
) {
    let entries = ContextMenuState::entries(settings);
    let win_size = Size::new(pixmap.width() as f64, pixmap.height() as f64);
    let (rect, row_h) = menu_state.layout_rect(win_size, scale_factor, entries.len());
    let s = scale_factor.max(1.0);

    // Panel background & border
    fill_rect(
        pixmap,
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        Color::rgba(24, 26, 32, 242),
    );
    // 1px border
    fill_rect(
        pixmap,
        rect.x,
        rect.y,
        rect.width,
        1.0 * s,
        Color::rgb(110, 120, 140),
    );
    fill_rect(
        pixmap,
        rect.x,
        rect.y + rect.height - 1.0 * s,
        rect.width,
        1.0 * s,
        Color::rgb(110, 120, 140),
    );
    fill_rect(
        pixmap,
        rect.x,
        rect.y,
        1.0 * s,
        rect.height,
        Color::rgb(110, 120, 140),
    );
    fill_rect(
        pixmap,
        rect.x + rect.width - 1.0 * s,
        rect.y,
        1.0 * s,
        rect.height,
        Color::rgb(110, 120, 140),
    );

    let font_scale = ((row_h * 0.55) / 7.0).max(1.0).floor() as i32;
    let glyph_h = 7 * font_scale;

    for (idx, entry) in entries.iter().enumerate() {
        let ry = rect.y + 3.0 * s + idx as f64 * row_h;
        if menu_state.hovered_index == Some(idx) {
            fill_rect(
                pixmap,
                rect.x + 2.0 * s,
                ry,
                rect.width - 4.0 * s,
                row_h,
                Color::rgb(55, 95, 185),
            );
        }

        let prefix = match entry.checked {
            Some(true) => "[X] ",
            Some(false) => "[ ] ",
            None => "    ",
        };
        let line = format!("{prefix}{}", entry.label);
        let text_x = (rect.x + 5.0 * s).round() as i32;
        let text_y = (ry + (row_h - glyph_h as f64) * 0.5).round() as i32;
        let max_x = (rect.x + rect.width - 4.0 * s).round() as i32;

        draw_bitmap_string(
            pixmap,
            &line,
            text_x,
            text_y,
            font_scale,
            max_x,
            Color::rgb(240, 244, 250),
        );
    }
}

fn fill_rect(pixmap: &mut Pixmap, x: f64, y: f64, w: f64, h: f64, color: Color) {
    if let Some(rect) = SkiaRect::from_xywh(x as f32, y as f32, w as f32, h as f32) {
        let mut paint = Paint::default();
        paint.set_color(color.to_tiny_skia());
        paint.anti_alias = false;
        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
    }
}

fn draw_bitmap_string(
    pixmap: &mut Pixmap,
    text: &str,
    start_x: i32,
    start_y: i32,
    scale: i32,
    max_x: i32,
    color: Color,
) {
    let w = pixmap.width() as i32;
    let h = pixmap.height() as i32;
    let premul = color.to_tiny_skia().premultiply().to_color_u8();
    let pixels = pixmap.pixels_mut();

    let mut cursor_x = start_x;
    for ch in text.chars() {
        let adv = 6 * scale;
        if cursor_x + 5 * scale > max_x {
            break;
        }
        let rows = glyph_5x7(ch);
        for (ry, bits) in rows.iter().enumerate() {
            for rx in 0..5 {
                if (bits >> (4 - rx)) & 1 == 1 {
                    for sy in 0..scale {
                        for sx in 0..scale {
                            let px = cursor_x + rx * scale + sx;
                            let py = start_y + (ry as i32) * scale + sy;
                            if px >= 0 && px < w && py >= 0 && py < h {
                                pixels[(py * w + px) as usize] = premul;
                            }
                        }
                    }
                }
            }
        }
        cursor_x += adv;
    }
}

/// Converts a `tiny_skia::Pixmap` (premultiplied RGBA8) into `softbuffer`'s `0xAARRGGBB` `u32` slice.
pub fn pixmap_to_softbuffer_u32(pixmap: &Pixmap, dst: &mut [u32]) {
    let src = pixmap.pixels();
    let len = src.len().min(dst.len());
    for (out, px) in dst[..len].iter_mut().zip(&src[..len]) {
        let r = px.red() as u32;
        let g = px.green() as u32;
        let b = px.blue() as u32;
        let a = px.alpha() as u32;
        *out = (a << 24) | (r << 16) | (g << 8) | b;
    }
}

/// Builds a 1-bit X11 `XYBitmap` mask (LSB-first per byte, rows padded to whole bytes)
/// from the alpha channel of `pixmap` (any pixel with `alpha >= threshold` is `1`).
pub fn build_xshape_bitmask(pixmap: &Pixmap, alpha_threshold: u8) -> Vec<u8> {
    let w = pixmap.width() as usize;
    let h = pixmap.height() as usize;
    let stride = w.div_ceil(8);
    let mut mask = vec![0u8; stride * h];
    let pixels = pixmap.pixels();

    for y in 0..h {
        let row_offset = y * stride;
        let px_row = y * w;
        for x in 0..w {
            if pixels[px_row + x].alpha() >= alpha_threshold {
                mask[row_offset + (x >> 3)] |= 1u8 << (x & 7);
            }
        }
    }

    mask
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_classic_shaped_frame_has_transparent_corners_and_opaque_eyes() {
        let settings = Settings::default();
        let layout = EyesLayout::compute(Size::new(150.0, 100.0), None, 0.22);
        let pixmap = render_eyes_frame(
            &settings,
            150,
            100,
            1.0,
            layout.left.center,
            layout.right.center,
            1.0,
            None,
        );

        // Top-left corner (0, 0) is outside both eyes -> transparent
        let corner = pixmap.pixel(0, 0).unwrap();
        assert_eq!(corner.alpha(), 0);

        // Left eye center has the black pupil
        let lcx = layout.left.center.x.round() as u32;
        let lcy = layout.left.center.y.round() as u32;
        let center_px = pixmap.pixel(lcx, lcy).unwrap();
        assert_eq!(center_px.alpha(), 255);
        assert_eq!(center_px.red(), 0);
        assert_eq!(center_px.green(), 0);
        assert_eq!(center_px.blue(), 0);

        // Sclera region (between pupil and outline) is white
        let sclera_x = (layout.left.center.x + layout.left.inner_rx * 0.6).round() as u32;
        let sclera_px = pixmap.pixel(sclera_x, lcy).unwrap();
        assert_eq!(sclera_px.alpha(), 255);
        assert_eq!(sclera_px.red(), 255);
        assert_eq!(sclera_px.green(), 255);
        assert_eq!(sclera_px.blue(), 255);
    }

    #[test]
    fn test_render_unshaped_fills_corners_with_background() {
        let mut settings = Settings::default();
        settings.appearance.shaped = false;
        let layout = EyesLayout::compute(Size::new(150.0, 100.0), None, 0.22);
        let pixmap = render_eyes_frame(
            &settings,
            150,
            100,
            1.0,
            layout.left.center,
            layout.right.center,
            1.0,
            None,
        );

        let corner = pixmap.pixel(0, 0).unwrap();
        assert_eq!(corner.alpha(), 255);
        assert_eq!(corner.red(), 255);
    }

    #[test]
    fn test_xshape_bitmask_generation() {
        let settings = Settings::default();
        let layout = EyesLayout::compute(Size::new(160.0, 100.0), None, 0.22);
        let pixmap = render_eyes_frame(
            &settings,
            160,
            100,
            1.0,
            layout.left.center,
            layout.right.center,
            1.0,
            None,
        );
        let mask = build_xshape_bitmask(&pixmap, 128);
        assert_eq!(mask.len(), (160 / 8) * 100);
        // Byte at (0, 0) should be 0
        assert_eq!(mask[0], 0);
        // Byte at left eye center should be non-zero
        let lcx = layout.left.center.x.round() as usize;
        let lcy = layout.left.center.y.round() as usize;
        let byte = mask[lcy * 20 + (lcx >> 3)];
        assert_ne!(byte & (1 << (lcx & 7)), 0);
    }
}
