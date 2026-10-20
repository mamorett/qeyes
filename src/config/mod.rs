pub mod store;
pub mod xrm;

use crate::cli::CliArgs;
use crate::config::xrm::XResourceOverrides;
use crate::geometry::XGeometry;
use crate::render::theme::{lookup_theme, Color};
use serde::{Deserialize, Serialize};

pub const CURRENT_CONFIG_VERSION: u32 = 1;
pub const DEFAULT_WIDTH: u32 = 150;
pub const DEFAULT_HEIGHT: u32 = 100;

/// Complete persisted and runtime configuration for `qeyes`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    pub window: WindowSettings,
    pub appearance: AppearanceSettings,
    pub behavior: BehaviorSettings,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowSettings {
    pub width: u32,
    pub height: u32,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub decorations: bool,
    pub always_on_top: bool,
    pub click_through: bool,
    pub lock_aspect: bool,
    pub title: String,
    pub app_name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearanceSettings {
    pub theme: String,
    pub foreground: Option<String>,
    pub background: Option<String>,
    pub outline: Option<String>,
    pub center: Option<String>,
    pub outline_width: Option<f64>,
    pub pupil_scale: f64,
    pub reverse_video: bool,
    pub shaped: bool,
    pub antialias: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BehaviorSettings {
    pub blink: bool,
    pub smooth: bool,
    pub classic_projection: bool,
    pub tray: bool,
    pub autostart: bool,
}

/// Resolved RGBA colors ready for the renderer after applying theme, explicit overrides,
/// and reverse video (`-rv`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedColors {
    pub foreground: Color,
    pub background: Color,
    pub outline: Color,
    pub center: Color,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: CURRENT_CONFIG_VERSION,
            window: WindowSettings::default(),
            appearance: AppearanceSettings::default(),
            behavior: BehaviorSettings::default(),
        }
    }
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            x: None,
            y: None,
            decorations: false,
            always_on_top: false,
            click_through: false,
            lock_aspect: false,
            title: "qeyes".to_string(),
            app_name: "qeyes".to_string(),
        }
    }
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme: "classic".to_string(),
            foreground: None,
            background: None,
            outline: None,
            center: None,
            outline_width: None,
            pupil_scale: 0.22,
            reverse_video: false,
            shaped: true,
            antialias: true,
        }
    }
}

impl Default for BehaviorSettings {
    fn default() -> Self {
        Self {
            blink: false,
            smooth: false,
            classic_projection: true,
            tray: false,
            autostart: false,
        }
    }
}

impl Settings {
    /// Performs schema migration and bounds clamping on loaded settings.
    pub fn migrate_and_validate(&mut self) {
        if self.version == 0 {
            self.version = CURRENT_CONFIG_VERSION;
        }
        self.window.width = self.window.width.clamp(48, 8192);
        self.window.height = self.window.height.clamp(32, 8192);
        self.appearance.pupil_scale = self.appearance.pupil_scale.clamp(0.05, 0.85);
        if let Some(bw) = self.appearance.outline_width {
            self.appearance.outline_width = Some(bw.clamp(0.5, 100.0));
        }
        if lookup_theme(&self.appearance.theme).is_none() {
            self.appearance.theme = "classic".to_string();
        }
    }

    /// Resolves the effective colors taking into account:
    /// 1. Base theme palette
    /// 2. Explicit `foreground` / `background` / `outline` / `center` overrides
    /// 3. Reverse video (`-rv`)
    pub fn resolved_colors(&self) -> ResolvedColors {
        let palette = lookup_theme(&self.appearance.theme)
            .unwrap_or(&crate::render::theme::BUILTIN_THEMES[0]);

        let fg_override = self
            .appearance
            .foreground
            .as_deref()
            .and_then(|s| Color::parse(s).ok());
        let bg_override = self
            .appearance
            .background
            .as_deref()
            .and_then(|s| Color::parse(s).ok());
        let outline_override = self
            .appearance
            .outline
            .as_deref()
            .and_then(|s| Color::parse(s).ok());
        let center_override = self
            .appearance
            .center
            .as_deref()
            .and_then(|s| Color::parse(s).ok());

        let mut fg = fg_override.unwrap_or(palette.foreground);
        let mut bg = bg_override.unwrap_or(palette.background);

        if self.appearance.reverse_video {
            std::mem::swap(&mut fg, &mut bg);
        }

        // In xeyes, if -fg is specified (or reversed via -rv), outline and center default to fg
        // unless -outline / -center were explicitly set.
        let outline = match outline_override {
            Some(c) => {
                if self.appearance.reverse_video && self.appearance.outline.is_none() {
                    fg
                } else {
                    c
                }
            }
            None => {
                if fg_override.is_some() || self.appearance.reverse_video {
                    fg
                } else {
                    palette.outline
                }
            }
        };

        let center = match center_override {
            Some(c) => c,
            None => {
                if fg_override.is_some() || self.appearance.reverse_video {
                    fg
                } else {
                    palette.center
                }
            }
        };

        ResolvedColors {
            foreground: fg,
            background: bg,
            outline,
            center,
        }
    }

    /// Applies X11 resource overrides (`RESOURCE_MANAGER`, `~/.Xresources`, `-xrm`).
    pub fn apply_xrm(&mut self, xrm: &XResourceOverrides) -> Option<XGeometry> {
        if let Some(c) = xrm.foreground {
            self.appearance.foreground = Some(c.to_hex());
        }
        if let Some(c) = xrm.background {
            self.appearance.background = Some(c.to_hex());
        }
        if let Some(c) = xrm.outline {
            self.appearance.outline = Some(c.to_hex());
        }
        if let Some(c) = xrm.center {
            self.appearance.center = Some(c.to_hex());
        }
        if let Some(rv) = xrm.reverse_video {
            self.appearance.reverse_video = rv;
        }
        if let Some(shaped) = xrm.shaped {
            self.appearance.shaped = shaped;
        }
        if let Some(render) = xrm.render {
            self.appearance.antialias = render;
        }
        if let Some(bw) = xrm.border_width {
            if bw > 0.0 {
                self.appearance.outline_width = Some(bw);
            }
        }
        if let Some(ref title) = xrm.title {
            self.window.title = title.clone();
        }
        if let Some(geom) = xrm.geometry {
            if let Some(w) = geom.width {
                self.window.width = w.clamp(48, 8192);
            }
            if let Some(h) = geom.height {
                self.window.height = h.clamp(32, 8192);
            }
        }
        xrm.geometry
    }

    /// Applies CLI options on top of current settings and returns any one-run `XGeometry`
    /// override so the caller can resolve negative screen offsets once monitor dimensions are known.
    pub fn apply_cli(&mut self, cli: &CliArgs) -> Result<Option<XGeometry>, String> {
        if let Some(ref name) = cli.name {
            self.window.app_name = name.clone();
        }
        if let Some(ref title) = cli.title {
            self.window.title = title.clone();
        }
        if let Some(ref theme) = cli.theme {
            let pal = lookup_theme(theme).ok_or_else(|| {
                format!(
                    "Unknown theme '{theme}'. Valid themes: classic, dark, nord, neon, solarized, bloodshot, cartoon, monochrome"
                )
            })?;
            self.appearance.theme = pal.name.to_string();
            // Switching theme via CLI clears custom per-channel overrides unless also passed on CLI
            self.appearance.foreground = None;
            self.appearance.background = None;
            self.appearance.outline = None;
            self.appearance.center = None;
            if let Some(ps) = pal.pupil_scale {
                self.appearance.pupil_scale = ps;
            }
        }

        if let Some(ref fg) = cli.foreground {
            let c = Color::parse(fg)?;
            self.appearance.foreground = Some(c.to_hex());
        }
        if let Some(ref bg) = cli.background {
            let c = Color::parse(bg)?;
            self.appearance.background = Some(c.to_hex());
        }
        if let Some(ref outline) = cli.outline {
            let c = Color::parse(outline)?;
            self.appearance.outline = Some(c.to_hex());
        }
        if let Some(ref center) = cli.center {
            let c = Color::parse(center)?;
            self.appearance.center = Some(c.to_hex());
        }
        if let Some(bw) = cli.borderwidth {
            if bw > 0.0 {
                self.appearance.outline_width = Some(bw);
            }
        }

        if cli.no_rv {
            self.appearance.reverse_video = false;
        } else if cli.rv {
            self.appearance.reverse_video = true;
        }

        if cli.no_shape {
            self.appearance.shaped = false;
        } else if cli.shape {
            self.appearance.shaped = true;
        }

        if cli.no_render {
            self.appearance.antialias = false;
        } else if cli.render {
            self.appearance.antialias = true;
        }

        if cli.always_on_top {
            self.window.always_on_top = true;
        }
        if cli.click_through {
            self.window.click_through = true;
        }
        if cli.decorations {
            self.window.decorations = true;
        }
        if cli.lock_aspect {
            self.window.lock_aspect = true;
        }
        if cli.blink {
            self.behavior.blink = true;
        }
        if cli.smooth {
            self.behavior.smooth = true;
        }
        if cli.no_tray {
            self.behavior.tray = false;
        } else if cli.tray {
            self.behavior.tray = true;
        }

        let mut parsed_geom = None;
        if let Some(ref geom_str) = cli.geometry {
            let g = XGeometry::parse(geom_str)?;
            if let Some(w) = g.width {
                self.window.width = w.clamp(48, 8192);
            }
            if let Some(h) = g.height {
                self.window.height = h.clamp(32, 8192);
            }
            parsed_geom = Some(g);
        }

        Ok(parsed_geom)
    }

    /// Sets a named built-in theme and resets explicit per-color overrides.
    pub fn set_theme(&mut self, theme_name: &str) {
        if let Some(pal) = lookup_theme(theme_name) {
            self.appearance.theme = pal.name.to_string();
            self.appearance.foreground = None;
            self.appearance.background = None;
            self.appearance.outline = None;
            self.appearance.center = None;
            if let Some(ps) = pal.pupil_scale {
                self.appearance.pupil_scale = ps;
            } else {
                self.appearance.pupil_scale = 0.22;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_precedence_defaults_config_xrm_cli() {
        let mut settings = Settings::default();
        // 1. Config sets foreground to blue
        settings.appearance.foreground = Some("#0000ff".to_string());

        // 2. XRM overrides foreground to green and sets reverseVideo
        let mut xrm = XResourceOverrides::default();
        xrm.parse_lines("Eyes.foreground: green\nEyes.reverseVideo: true", "qeyes")
            .unwrap();
        settings.apply_xrm(&xrm);
        assert_eq!(settings.appearance.foreground.as_deref(), Some("#00ff00"));
        assert!(settings.appearance.reverse_video);

        // 3. CLI overrides foreground to red and +rv disables reverse video
        let cli = CliArgs::parse_from_raw(["qeyes", "-fg", "red", "+rv"]).unwrap();
        settings.apply_cli(&cli).unwrap();

        let colors = settings.resolved_colors();
        assert_eq!(colors.foreground, Color::rgb(255, 0, 0));
        assert_eq!(colors.background, Color::rgb(255, 255, 255));
        assert!(!settings.appearance.reverse_video);
    }

    #[test]
    fn test_reverse_video_swaps_fg_and_bg() {
        let mut s = Settings::default();
        let normal = s.resolved_colors();
        assert_eq!(normal.foreground, Color::BLACK);
        assert_eq!(normal.background, Color::WHITE);

        s.appearance.reverse_video = true;
        let rev = s.resolved_colors();
        assert_eq!(rev.foreground, Color::WHITE);
        assert_eq!(rev.background, Color::BLACK);
        assert_eq!(rev.outline, Color::WHITE);
        assert_eq!(rev.center, Color::WHITE);
    }
}
