use crate::config::Settings;
use crate::geometry::{Point, Rect, Size};

/// Actions that can be triggered from the interactive context menu or keyboard shortcuts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    ToggleAlwaysOnTop,
    ToggleShaped,
    ToggleAntialias,
    ToggleLockAspect,
    ToggleBlink,
    ToggleSmooth,
    CycleTheme,
    ToggleAutostart,
    ResetSizeAndPos,
    Quit,
}

#[derive(Debug, Clone)]
pub struct MenuEntry {
    pub label: String,
    pub checked: Option<bool>,
    pub action: MenuAction,
}

/// Interactive in-window right-click context menu state.
#[derive(Debug, Clone, Default)]
pub struct ContextMenuState {
    pub visible: bool,
    pub anchor: Point,
    pub hovered_index: Option<usize>,
}

impl ContextMenuState {
    pub fn open(&mut self, pos: Point) {
        self.visible = true;
        self.anchor = pos;
        self.hovered_index = None;
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.hovered_index = None;
    }

    pub fn entries(settings: &Settings) -> Vec<MenuEntry> {
        vec![
            MenuEntry {
                label: "Always on Top (A)".to_string(),
                checked: Some(settings.window.always_on_top),
                action: MenuAction::ToggleAlwaysOnTop,
            },
            MenuEntry {
                label: "Shaped Window (S)".to_string(),
                checked: Some(settings.appearance.shaped),
                action: MenuAction::ToggleShaped,
            },
            MenuEntry {
                label: "Antialiasing".to_string(),
                checked: Some(settings.appearance.antialias),
                action: MenuAction::ToggleAntialias,
            },
            MenuEntry {
                label: "Lock Aspect".to_string(),
                checked: Some(settings.window.lock_aspect),
                action: MenuAction::ToggleLockAspect,
            },
            MenuEntry {
                label: "Blinking (B)".to_string(),
                checked: Some(settings.behavior.blink),
                action: MenuAction::ToggleBlink,
            },
            MenuEntry {
                label: "Smooth Track".to_string(),
                checked: Some(settings.behavior.smooth),
                action: MenuAction::ToggleSmooth,
            },
            MenuEntry {
                label: format!("Theme: {}", settings.appearance.theme),
                checked: None,
                action: MenuAction::CycleTheme,
            },
            MenuEntry {
                label: "Run at Login".to_string(),
                checked: Some(settings.behavior.autostart),
                action: MenuAction::ToggleAutostart,
            },
            MenuEntry {
                label: "Reset Size (0)".to_string(),
                checked: None,
                action: MenuAction::ResetSizeAndPos,
            },
            MenuEntry {
                label: "Quit (Esc/Q)".to_string(),
                checked: None,
                action: MenuAction::Quit,
            },
        ]
    }

    /// Computes the bounding rectangle and row height of the context menu in physical pixels.
    pub fn layout_rect(&self, win_size: Size, scale: f64, entry_count: usize) -> (Rect, f64) {
        let s = scale.max(1.0);
        let row_h = (16.0 * s).min((win_size.height - 4.0 * s).max(10.0) / (entry_count as f64));
        let row_h = row_h.max(8.0);
        let menu_w = (152.0 * s).min((win_size.width - 4.0 * s).max(40.0));
        let menu_h = (row_h * entry_count as f64 + 6.0 * s).min(win_size.height);

        let max_x = (win_size.width - menu_w - 2.0 * s).max(0.0);
        let min_x = (2.0 * s).min(max_x);
        let max_y = (win_size.height - menu_h - 2.0 * s).max(0.0);
        let min_y = (2.0 * s).min(max_y);

        let x = self.anchor.x.clamp(min_x, max_x);
        let y = self.anchor.y.clamp(min_y, max_y);

        (Rect::new(x, y, menu_w, menu_h), row_h)
    }

    /// Hit-tests a pointer position in physical pixels against the open menu.
    pub fn hit_test(
        &self,
        pt: Point,
        win_size: Size,
        scale: f64,
        entry_count: usize,
    ) -> Option<usize> {
        if !self.visible || entry_count == 0 {
            return None;
        }
        let s = scale.max(1.0);
        let (rect, row_h) = self.layout_rect(win_size, scale, entry_count);
        if pt.x < rect.x
            || pt.x > rect.x + rect.width
            || pt.y < rect.y + 3.0 * s
            || pt.y >= rect.y + rect.height - 3.0 * s
        {
            return None;
        }
        let idx = ((pt.y - (rect.y + 3.0 * s)) / row_h).floor() as usize;
        if idx < entry_count {
            Some(idx)
        } else {
            None
        }
    }
}

/// Renders a 5x7 ASCII character glyph into a pixel callback `(dx, dy)`.
pub fn glyph_5x7(ch: char) -> [u8; 7] {
    match ch.to_ascii_uppercase() {
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
        ],
        'D' => [
            0b11100, 0b10010, 0b10001, 0b10001, 0b10001, 0b10010, 0b11100,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        'J' => [
            0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001,
        ],
        'X' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
        ],
        '6' => [
            0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
        ],
        ':' => [
            0b00000, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100, 0b00000,
        ],
        '(' => [
            0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0b00100, 0b00010,
        ],
        ')' => [
            0b01000, 0b00100, 0b00010, 0b00010, 0b00010, 0b00100, 0b01000,
        ],
        '/' => [
            0b00001, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b10000,
        ],
        '-' => [
            0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
        ],
        _ => [0; 7],
    }
}
