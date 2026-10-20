use crate::config::store::save_settings;
use crate::config::{Settings, DEFAULT_HEIGHT, DEFAULT_WIDTH};
use crate::geometry::{
    clamp_rect_to_monitors, compute_pupil_position, EyesLayout, Point, Rect, Size, XGeometry,
};
use crate::platform::set_autostart;
use crate::pointer::{create_pointer_source, PointerSource};
use crate::render::theme::next_theme_name;
use crate::render::{pixmap_to_softbuffer_u32, render_eyes_frame};
use crate::ui::menu::{ContextMenuState, MenuAction};
use crate::window::apply_window_attributes;
use crate::window::resize::{direction_to_cursor_icon, hit_test_resize_grip, scale_logical_size};
#[cfg(target_os = "linux")]
use crate::window::shape_x11::X11ShapeManager;
use serde::Serialize;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize, PhysicalSize};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Window, WindowId};

/// JSON diagnostic payload emitted by `qeyes --dump-state`.
#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticState {
    pub window_width: u32,
    pub window_height: u32,
    pub pointer: Point,
    pub left_eye_center: Point,
    pub right_eye_center: Point,
    pub left_pupil: Point,
    pub right_pupil: Point,
    pub theme: String,
    pub shaped: bool,
    pub antialias: bool,
    pub reverse_video: bool,
    pub foreground: String,
    pub background: String,
}

/// Computes a single frame of state (using either `--test-pointer X,Y` or the live platform
/// `PointerSource`) for `--dump-state` verification.
pub fn compute_diagnostic_state(
    settings: &Settings,
    display_name: Option<&str>,
    test_pointer: Option<&str>,
) -> Result<DiagnosticState, String> {
    let w = settings.window.width.max(48);
    let h = settings.window.height.max(32);
    let layout = EyesLayout::compute(
        Size::new(w as f64, h as f64),
        settings.appearance.outline_width,
        settings.appearance.pupil_scale,
    );

    let pointer = if let Some(tp) = test_pointer {
        let (xs, ys) = tp
            .split_once(',')
            .ok_or_else(|| format!("Invalid --test-pointer '{tp}', expected 'X,Y'"))?;
        let x: f64 = xs
            .trim()
            .parse()
            .map_err(|_| format!("Invalid X in --test-pointer '{tp}'"))?;
        let y: f64 = ys
            .trim()
            .parse()
            .map_err(|_| format!("Invalid Y in --test-pointer '{tp}'"))?;
        Point::new(x, y)
    } else {
        let mut src = create_pointer_source(display_name);
        let global = src.global_position().unwrap_or(Point::new(0.0, 0.0));
        let wx = settings.window.x.unwrap_or(0) as f64;
        let wy = settings.window.y.unwrap_or(0) as f64;
        Point::new(global.x - wx, global.y - wy)
    };

    let left_pupil =
        compute_pupil_position(&layout.left, pointer, settings.behavior.classic_projection);
    let right_pupil =
        compute_pupil_position(&layout.right, pointer, settings.behavior.classic_projection);
    let colors = settings.resolved_colors();

    Ok(DiagnosticState {
        window_width: w,
        window_height: h,
        pointer,
        left_eye_center: layout.left.center,
        right_eye_center: layout.right.center,
        left_pupil,
        right_pupil,
        theme: settings.appearance.theme.clone(),
        shaped: settings.appearance.shaped,
        antialias: settings.appearance.antialias,
        reverse_video: settings.appearance.reverse_video,
        foreground: colors.foreground.to_hex(),
        background: colors.background.to_hex(),
    })
}

pub struct QEyesApp {
    settings: Settings,
    config_path: PathBuf,
    cli_geometry: Option<XGeometry>,
    force_xshape: bool,
    window: Option<Rc<Window>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    pointer_source: Box<dyn PointerSource>,
    #[cfg(target_os = "linux")]
    x11_shape: Option<X11ShapeManager>,
    last_cursor: Point,
    left_pupil: Point,
    right_pupil: Point,
    last_pointer_move: Instant,
    next_blink_at: Instant,
    blink_started_at: Option<Instant>,
    last_blink_openness: f64,
    menu: ContextMenuState,
    modifiers: ModifiersState,
    config_dirty_since: Option<Instant>,
    initial_position_applied: bool,
}

impl QEyesApp {
    pub fn new(
        settings: Settings,
        config_path: PathBuf,
        cli_geometry: Option<XGeometry>,
        display_name: Option<String>,
        force_xshape: bool,
    ) -> Self {
        let now = Instant::now();
        let pointer_source = create_pointer_source(display_name.as_deref());
        #[cfg(target_os = "linux")]
        let x11_shape = X11ShapeManager::new(display_name.as_deref());

        let layout = EyesLayout::compute(
            Size::new(settings.window.width as f64, settings.window.height as f64),
            settings.appearance.outline_width,
            settings.appearance.pupil_scale,
        );

        Self {
            settings,
            config_path,
            cli_geometry,
            force_xshape,
            window: None,
            surface: None,
            pointer_source,
            #[cfg(target_os = "linux")]
            x11_shape,
            last_cursor: Point::new(0.0, 0.0),
            left_pupil: layout.left.center,
            right_pupil: layout.right.center,
            last_pointer_move: now,
            next_blink_at: now + Duration::from_millis(4200),
            blink_started_at: None,
            last_blink_openness: 1.0,
            menu: ContextMenuState::default(),
            modifiers: ModifiersState::empty(),
            config_dirty_since: None,
            initial_position_applied: false,
        }
    }

    fn mark_config_dirty(&mut self) {
        if self.config_dirty_since.is_none() {
            self.config_dirty_since = Some(Instant::now());
        }
    }

    fn flush_config_now(&mut self) {
        if self.config_dirty_since.take().is_some() {
            if let Err(e) = save_settings(&self.config_path, &self.settings) {
                tracing::warn!("Failed to save config: {e}");
            }
        }
    }

    fn current_physical_size(&self) -> (u32, u32, f64) {
        if let Some(ref win) = self.window {
            let scale = win.scale_factor().max(1.0);
            let phys = win.inner_size();
            (phys.width.max(1), phys.height.max(1), scale)
        } else {
            (
                self.settings.window.width.max(1),
                self.settings.window.height.max(1),
                1.0,
            )
        }
    }

    fn compute_blink_openness(&mut self, now: Instant) -> f64 {
        if !self.settings.behavior.blink {
            self.blink_started_at = None;
            return 1.0;
        }

        // Sleepy eyes when pointer hasn't moved for 5 minutes
        let idle_secs = now.duration_since(self.last_pointer_move).as_secs_f64();
        let base_openness = if idle_secs > 300.0 { 0.45 } else { 1.0 };

        if self.blink_started_at.is_none() && now >= self.next_blink_at {
            self.blink_started_at = Some(now);
        }

        if let Some(start) = self.blink_started_at {
            let elapsed_ms = now.duration_since(start).as_secs_f64() * 1000.0;
            let blink_duration_ms = 150.0;
            if elapsed_ms >= blink_duration_ms {
                self.blink_started_at = None;
                // Deterministic pseudo-random interval between 3.2s and 6.8s based on pupil coords
                let jitter = ((self.left_pupil.x.abs() * 37.0 + self.right_pupil.y.abs() * 19.0)
                    as u64)
                    % 3600;
                self.next_blink_at = now + Duration::from_millis(3200 + jitter);
                return base_openness;
            }
            // Triangle wave 1.0 -> 0.0 -> 1.0
            let phase = (elapsed_ms / (blink_duration_ms * 0.5) - 1.0).abs();
            return (phase * base_openness).clamp(0.0, 1.0);
        }

        base_openness
    }

    fn update_pointer_and_pupils(&mut self, now: Instant) -> bool {
        let Some(ref win) = self.window else {
            return false;
        };

        if let Some(pt) = self.pointer_source.cursor_in_window(win) {
            if pt.distance_to(self.last_cursor) >= 0.5 {
                self.last_cursor = pt;
                self.last_pointer_move = now;
            }
        }

        let (w, h, scale) = self.current_physical_size();
        let outline_override = self.settings.appearance.outline_width.map(|bw| bw * scale);
        let layout = EyesLayout::compute(
            Size::new(w as f64, h as f64),
            outline_override,
            self.settings.appearance.pupil_scale,
        );

        let target_left = compute_pupil_position(
            &layout.left,
            self.last_cursor,
            self.settings.behavior.classic_projection,
        );
        let target_right = compute_pupil_position(
            &layout.right,
            self.last_cursor,
            self.settings.behavior.classic_projection,
        );

        let (new_left, new_right) = if self.settings.behavior.smooth {
            (
                self.left_pupil.lerp(target_left, 0.28),
                self.right_pupil.lerp(target_right, 0.28),
            )
        } else {
            (target_left, target_right)
        };

        let blink_openness = self.compute_blink_openness(now);

        let changed = new_left.distance_to(self.left_pupil) >= 0.2
            || new_right.distance_to(self.right_pupil) >= 0.2
            || (blink_openness - self.last_blink_openness).abs() >= 0.02;

        self.left_pupil = new_left;
        self.right_pupil = new_right;
        self.last_blink_openness = blink_openness;

        changed
    }

    fn execute_menu_action(&mut self, action: MenuAction, event_loop: &ActiveEventLoop) {
        match action {
            MenuAction::ToggleAlwaysOnTop => {
                self.settings.window.always_on_top = !self.settings.window.always_on_top;
            }
            MenuAction::ToggleShaped => {
                self.settings.appearance.shaped = !self.settings.appearance.shaped;
            }
            MenuAction::ToggleAntialias => {
                self.settings.appearance.antialias = !self.settings.appearance.antialias;
            }
            MenuAction::ToggleLockAspect => {
                self.settings.window.lock_aspect = !self.settings.window.lock_aspect;
            }
            MenuAction::ToggleBlink => {
                self.settings.behavior.blink = !self.settings.behavior.blink;
            }
            MenuAction::ToggleSmooth => {
                self.settings.behavior.smooth = !self.settings.behavior.smooth;
            }
            MenuAction::CycleTheme => {
                let next = next_theme_name(&self.settings.appearance.theme);
                self.settings.set_theme(next);
            }
            MenuAction::ToggleAutostart => {
                let target = !self.settings.behavior.autostart;
                if let Err(e) = set_autostart(target) {
                    tracing::warn!("Failed to update autostart: {e}");
                } else {
                    self.settings.behavior.autostart = target;
                }
            }
            MenuAction::ResetSizeAndPos => {
                self.resize_window_to_logical(DEFAULT_WIDTH, DEFAULT_HEIGHT);
            }
            MenuAction::Quit => {
                self.flush_config_now();
                event_loop.exit();
                return;
            }
        }

        if let Some(ref win) = self.window {
            apply_window_attributes(win, &self.settings);
            win.request_redraw();
        }
        self.mark_config_dirty();
    }

    fn resize_window_to_logical(&mut self, width: u32, height: u32) {
        self.settings.window.width = width.clamp(48, 8192);
        self.settings.window.height = height.clamp(32, 8192);
        if let Some(ref win) = self.window {
            let _ = win.request_inner_size(LogicalSize::new(
                self.settings.window.width,
                self.settings.window.height,
            ));
            win.request_redraw();
        }
        self.mark_config_dirty();
    }

    fn scale_window_by(&mut self, factor: f64) {
        let current = Size::new(
            self.settings.window.width as f64,
            self.settings.window.height as f64,
        );
        let max_size = self
            .window
            .as_ref()
            .and_then(|w| w.current_monitor())
            .map(|m| {
                let s = m.scale_factor().max(1.0);
                let sz = m.size();
                Size::new(sz.width as f64 / s, sz.height as f64 / s)
            })
            .unwrap_or(Size::new(3840.0, 2160.0));

        let (nw, nh) =
            scale_logical_size(current, factor, max_size, self.settings.window.lock_aspect);
        self.resize_window_to_logical(nw, nh);
    }

    fn render_and_present(&mut self) {
        let Some(ref win) = self.window else {
            return;
        };
        let Some(ref mut surface) = self.surface else {
            return;
        };

        let phys = win.inner_size();
        let w = phys.width.max(1);
        let h = phys.height.max(1);
        let scale = win.scale_factor().max(1.0);

        let Some(nz_w) = NonZeroU32::new(w) else {
            return;
        };
        let Some(nz_h) = NonZeroU32::new(h) else {
            return;
        };

        if surface.resize(nz_w, nz_h).is_err() {
            return;
        }

        let pixmap = render_eyes_frame(
            &self.settings,
            w,
            h,
            scale,
            self.left_pupil,
            self.right_pupil,
            self.last_blink_openness,
            Some(&self.menu),
        );

        #[cfg(target_os = "linux")]
        if let Some(ref shape_mgr) = self.x11_shape {
            shape_mgr.update_window_shapes(
                win,
                &pixmap,
                self.settings.appearance.shaped,
                self.settings.window.click_through,
                self.force_xshape,
            );
        }

        if let Ok(mut buffer) = surface.buffer_mut() {
            pixmap_to_softbuffer_u32(&pixmap, &mut buffer);
            let _ = buffer.present();
        }
    }
}

impl ApplicationHandler for QEyesApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        // Gather connected monitors for geometry offset resolution and off-screen clamping
        let mut monitor_rects = Vec::new();
        for mon in event_loop.available_monitors() {
            let s = mon.scale_factor().max(1.0);
            let pos = mon.position();
            let sz = mon.size();
            monitor_rects.push(Rect::new(
                pos.x as f64 / s,
                pos.y as f64 / s,
                sz.width as f64 / s,
                sz.height as f64 / s,
            ));
        }

        let primary_rect = event_loop
            .primary_monitor()
            .map(|mon| {
                let s = mon.scale_factor().max(1.0);
                let pos = mon.position();
                let sz = mon.size();
                Rect::new(
                    pos.x as f64 / s,
                    pos.y as f64 / s,
                    sz.width as f64 / s,
                    sz.height as f64 / s,
                )
            })
            .or_else(|| monitor_rects.first().copied())
            .unwrap_or(Rect::new(0.0, 0.0, 1920.0, 1080.0));

        if monitor_rects.is_empty() {
            monitor_rects.push(primary_rect);
        }

        let win_w = self.settings.window.width as f64;
        let win_h = self.settings.window.height as f64;

        let target_pos: Option<(i32, i32)> = if let Some(geom) = self.cli_geometry {
            geom.resolve_position(primary_rect.width, primary_rect.height, win_w, win_h)
                .map(|(x, y)| {
                    (
                        (primary_rect.x + x).round() as i32,
                        (primary_rect.y + y).round() as i32,
                    )
                })
        } else if let (Some(saved_x), Some(saved_y)) =
            (self.settings.window.x, self.settings.window.y)
        {
            let clamped = clamp_rect_to_monitors(
                Rect::new(saved_x as f64, saved_y as f64, win_w, win_h),
                &monitor_rects,
                primary_rect,
            );
            self.settings.window.width = clamped.width.round() as u32;
            self.settings.window.height = clamped.height.round() as u32;
            Some((clamped.x.round() as i32, clamped.y.round() as i32))
        } else {
            None
        };

        let mut attrs = Window::default_attributes()
            .with_title(&self.settings.window.title)
            .with_inner_size(LogicalSize::new(
                self.settings.window.width,
                self.settings.window.height,
            ))
            .with_min_inner_size(LogicalSize::new(48u32, 32u32))
            .with_transparent(true)
            .with_decorations(self.settings.window.decorations);

        if let Some((px, py)) = target_pos {
            attrs = attrs.with_position(LogicalPosition::new(px, py));
        }

        #[cfg(target_os = "linux")]
        {
            use winit::platform::x11::WindowAttributesExtX11;
            attrs = attrs.with_name(&self.settings.window.app_name, "Eyes");
        }

        let win = match event_loop.create_window(attrs) {
            Ok(w) => Rc::new(w),
            Err(e) => {
                tracing::error!("Failed to create window: {e}");
                event_loop.exit();
                return;
            }
        };

        apply_window_attributes(&win, &self.settings);

        if let Some((px, py)) = target_pos {
            win.set_outer_position(LogicalPosition::new(px, py));
        }

        let context = match softbuffer::Context::new(win.clone()) {
            Ok(ctx) => ctx,
            Err(e) => {
                tracing::error!("Failed to create softbuffer context: {e}");
                event_loop.exit();
                return;
            }
        };

        let surface = match softbuffer::Surface::new(&context, win.clone()) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to create softbuffer surface: {e}");
                event_loop.exit();
                return;
            }
        };

        self.window = Some(win.clone());
        self.surface = Some(surface);
        self.initial_position_applied = true;

        self.update_pointer_and_pupils(Instant::now());
        win.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.flush_config_now();
                event_loop.exit();
            }
            WindowEvent::ModifiersChanged(mods) => {
                self.modifiers = mods.state();
            }
            WindowEvent::Resized(PhysicalSize { width, height }) => {
                if width > 0 && height > 0 {
                    let scale = self
                        .window
                        .as_ref()
                        .map(|w| w.scale_factor().max(1.0))
                        .unwrap_or(1.0);
                    let lw = ((width as f64) / scale).round() as u32;
                    let lh = ((height as f64) / scale).round() as u32;
                    if lw >= 48 && lh >= 32 {
                        self.settings.window.width = lw;
                        self.settings.window.height = lh;
                        if self.cli_geometry.is_none() {
                            self.mark_config_dirty();
                        }
                    }
                    self.update_pointer_and_pupils(Instant::now());
                    if let Some(ref win) = self.window {
                        win.request_redraw();
                    }
                }
            }
            WindowEvent::Moved(phys_pos) => {
                if self.initial_position_applied && self.cli_geometry.is_none() {
                    let scale = self
                        .window
                        .as_ref()
                        .map(|w| w.scale_factor().max(1.0))
                        .unwrap_or(1.0);
                    self.settings.window.x = Some(((phys_pos.x as f64) / scale).round() as i32);
                    self.settings.window.y = Some(((phys_pos.y as f64) / scale).round() as i32);
                    self.mark_config_dirty();
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(ref win) = self.window {
                    win.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let pt = Point::new(position.x, position.y);
                self.last_cursor = pt;
                self.last_pointer_move = Instant::now();

                let (w, h, scale) = self.current_physical_size();
                let win_size = Size::new(w as f64, h as f64);

                if self.menu.visible {
                    let entries = ContextMenuState::entries(&self.settings);
                    let new_hover = self.menu.hit_test(pt, win_size, scale, entries.len());
                    if new_hover != self.menu.hovered_index {
                        self.menu.hovered_index = new_hover;
                        if let Some(ref win) = self.window {
                            win.request_redraw();
                        }
                    }
                } else if let Some(ref win) = self.window {
                    let dir = hit_test_resize_grip(pt, win_size, scale);
                    win.set_cursor(direction_to_cursor_icon(dir));
                }

                if self.update_pointer_and_pupils(Instant::now()) {
                    if let Some(ref win) = self.window {
                        win.request_redraw();
                    }
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } => {
                let (w, h, scale) = self.current_physical_size();
                let win_size = Size::new(w as f64, h as f64);

                match button {
                    MouseButton::Right => {
                        if self.menu.visible {
                            self.menu.close();
                        } else {
                            self.menu.open(self.last_cursor);
                        }
                        if let Some(ref win) = self.window {
                            win.request_redraw();
                        }
                    }
                    MouseButton::Left => {
                        if self.menu.visible {
                            let entries = ContextMenuState::entries(&self.settings);
                            if let Some(idx) =
                                self.menu
                                    .hit_test(self.last_cursor, win_size, scale, entries.len())
                            {
                                let action = entries[idx].action;
                                self.menu.close();
                                self.execute_menu_action(action, event_loop);
                            } else {
                                self.menu.close();
                                if let Some(ref win) = self.window {
                                    win.request_redraw();
                                }
                            }
                            return;
                        }

                        // User interacted with window -> subsequent moves/resizes should be saved
                        self.cli_geometry = None;

                        if let Some(ref win) = self.window {
                            if let Some(dir) =
                                hit_test_resize_grip(self.last_cursor, win_size, scale)
                            {
                                if win.drag_resize_window(dir).is_err() {
                                    let _ = win.drag_window();
                                }
                            } else {
                                let _ = win.drag_window();
                            }
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let dy = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y as f64,
                    MouseScrollDelta::PixelDelta(p) => p.y / 24.0,
                };
                if dy.abs() > 0.01 {
                    self.cli_geometry = None;
                    let factor = if dy > 0.0 { 1.10 } else { 0.90 };
                    self.scale_window_by(factor);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }

                match &event.logical_key {
                    Key::Named(NamedKey::Escape) => {
                        if self.menu.visible {
                            self.menu.close();
                            if let Some(ref win) = self.window {
                                win.request_redraw();
                            }
                        } else {
                            self.flush_config_now();
                            event_loop.exit();
                        }
                    }
                    Key::Character(ch) => match ch.as_str() {
                        "q" | "Q" => {
                            self.flush_config_now();
                            event_loop.exit();
                        }
                        "+" | "=" => {
                            self.cli_geometry = None;
                            self.scale_window_by(1.10);
                        }
                        "-" | "_" => {
                            self.cli_geometry = None;
                            self.scale_window_by(0.90);
                        }
                        "0" => {
                            self.cli_geometry = None;
                            self.execute_menu_action(MenuAction::ResetSizeAndPos, event_loop);
                        }
                        "a" | "A" => {
                            self.execute_menu_action(MenuAction::ToggleAlwaysOnTop, event_loop);
                        }
                        "s" | "S" => {
                            self.execute_menu_action(MenuAction::ToggleShaped, event_loop);
                        }
                        "b" | "B" => {
                            self.execute_menu_action(MenuAction::ToggleBlink, event_loop);
                        }
                        "t" | "T" => {
                            self.execute_menu_action(MenuAction::CycleTheme, event_loop);
                        }
                        "r" | "R" => {
                            self.settings.appearance.reverse_video =
                                !self.settings.appearance.reverse_video;
                            self.mark_config_dirty();
                            if let Some(ref win) = self.window {
                                win.request_redraw();
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => {
                self.render_and_present();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();

        if self.update_pointer_and_pupils(now) {
            if let Some(ref win) = self.window {
                win.request_redraw();
            }
        }

        // Flush debounced config writes after 500 ms
        if let Some(dirty_at) = self.config_dirty_since {
            if now.duration_since(dirty_at) >= Duration::from_millis(500) {
                self.flush_config_now();
            }
        }

        // Adaptive polling cadence:
        // - 60 Hz (16 ms) while pointer is moving (< 1s since last move) or blinking animation active
        // - 10 Hz (100 ms) between 1s and 10s of pointer inactivity
        // - 4 Hz (250 ms) after 10s of pointer inactivity
        let idle = now.duration_since(self.last_pointer_move);
        let interval = if self.blink_started_at.is_some() || idle < Duration::from_secs(1) {
            Duration::from_millis(16)
        } else if idle < Duration::from_secs(10) {
            Duration::from_millis(100)
        } else {
            Duration::from_millis(250)
        };

        event_loop.set_control_flow(ControlFlow::WaitUntil(now + interval));
    }
}
