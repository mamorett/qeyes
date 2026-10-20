use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// An RGBA8 color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn to_hex(self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
        }
    }

    pub fn to_tiny_skia(self) -> tiny_skia::Color {
        tiny_skia::Color::from_rgba8(self.r, self.g, self.b, self.a)
    }

    /// Parses an X11 or CSS color string:
    /// - `#rgb`, `#rrggbb`, `#rrrrggggbbbb`, `#rrggbbaa`
    /// - `rgb:r/g/b` (1 to 4 hex digits per channel, X11 specification)
    /// - Named X11 colors (case-insensitive, spaces/underscores/hyphens ignored, `grey`/`gray` normalized)
    pub fn parse(input: &str) -> Result<Self, String> {
        let s = input.trim();
        if s.is_empty() {
            return Err("Empty color string".to_string());
        }

        if let Some(hex) = s.strip_prefix('#') {
            return Self::parse_hex(hex, input);
        }

        if let Some(rgb_spec) = s.strip_prefix("rgb:").or_else(|| s.strip_prefix("RGB:")) {
            return Self::parse_x11_rgb_spec(rgb_spec, input);
        }

        if let Some(c) = lookup_x11_color(s) {
            return Ok(c);
        }

        Err(format!("Unknown color specification '{input}'"))
    }

    fn parse_hex(hex: &str, full: &str) -> Result<Self, String> {
        match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1], 16)
                    .map_err(|_| format!("Invalid hex color '{full}'"))?;
                let g = u8::from_str_radix(&hex[1..2], 16)
                    .map_err(|_| format!("Invalid hex color '{full}'"))?;
                let b = u8::from_str_radix(&hex[2..3], 16)
                    .map_err(|_| format!("Invalid hex color '{full}'"))?;
                Ok(Self::rgb(r * 17, g * 17, b * 17))
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16)
                    .map_err(|_| format!("Invalid hex color '{full}'"))?;
                let g = u8::from_str_radix(&hex[2..4], 16)
                    .map_err(|_| format!("Invalid hex color '{full}'"))?;
                let b = u8::from_str_radix(&hex[4..6], 16)
                    .map_err(|_| format!("Invalid hex color '{full}'"))?;
                Ok(Self::rgb(r, g, b))
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16)
                    .map_err(|_| format!("Invalid hex color '{full}'"))?;
                let g = u8::from_str_radix(&hex[2..4], 16)
                    .map_err(|_| format!("Invalid hex color '{full}'"))?;
                let b = u8::from_str_radix(&hex[4..6], 16)
                    .map_err(|_| format!("Invalid hex color '{full}'"))?;
                let a = u8::from_str_radix(&hex[6..8], 16)
                    .map_err(|_| format!("Invalid hex color '{full}'"))?;
                Ok(Self::rgba(r, g, b, a))
            }
            12 => {
                // 16-bit per channel X11 hex (#RRRRGGGGBBBB), take high byte
                let r = u8::from_str_radix(&hex[0..2], 16)
                    .map_err(|_| format!("Invalid hex color '{full}'"))?;
                let g = u8::from_str_radix(&hex[4..6], 16)
                    .map_err(|_| format!("Invalid hex color '{full}'"))?;
                let b = u8::from_str_radix(&hex[8..10], 16)
                    .map_err(|_| format!("Invalid hex color '{full}'"))?;
                Ok(Self::rgb(r, g, b))
            }
            _ => Err(format!("Unsupported hex color length in '{full}'")),
        }
    }

    fn parse_x11_rgb_spec(spec: &str, full: &str) -> Result<Self, String> {
        let parts: Vec<&str> = spec.split('/').collect();
        if parts.len() != 3 {
            return Err(format!("Invalid X11 rgb:r/g/b color '{full}'"));
        }
        let parse_channel = |part: &str| -> Result<u8, String> {
            if part.is_empty() || part.len() > 4 {
                return Err(format!("Invalid channel '{part}' in '{full}'"));
            }
            let val = u16::from_str_radix(part, 16)
                .map_err(|_| format!("Invalid hex channel '{part}' in '{full}'"))?;
            let max = (1u32 << (part.len() * 4)) - 1;
            Ok(((val as u32 * 255 + max / 2) / max) as u8)
        };
        Ok(Self::rgb(
            parse_channel(parts[0])?,
            parse_channel(parts[1])?,
            parse_channel(parts[2])?,
        ))
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl FromStr for Color {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Built-in visual themes for `qeyes`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ThemePalette {
    pub name: &'static str,
    pub foreground: Color,
    pub background: Color,
    pub outline: Color,
    pub center: Color,
    pub pupil_scale: Option<f64>,
}

pub const BUILTIN_THEMES: &[ThemePalette] = &[
    ThemePalette {
        name: "classic",
        foreground: Color::rgb(0, 0, 0),
        background: Color::rgb(255, 255, 255),
        outline: Color::rgb(0, 0, 0),
        center: Color::rgb(0, 0, 0),
        pupil_scale: None,
    },
    ThemePalette {
        name: "dark",
        foreground: Color::rgb(230, 234, 240),
        background: Color::rgb(32, 36, 44),
        outline: Color::rgb(130, 145, 170),
        center: Color::rgb(235, 240, 250),
        pupil_scale: None,
    },
    ThemePalette {
        name: "neon",
        foreground: Color::rgb(0, 255, 204),
        background: Color::rgb(14, 12, 28),
        outline: Color::rgb(255, 0, 144),
        center: Color::rgb(0, 255, 204),
        pupil_scale: None,
    },
    ThemePalette {
        name: "solarized",
        foreground: Color::rgb(7, 54, 66),
        background: Color::rgb(253, 246, 227),
        outline: Color::rgb(88, 110, 117),
        center: Color::rgb(38, 139, 210),
        pupil_scale: None,
    },
    ThemePalette {
        name: "bloodshot",
        foreground: Color::rgb(40, 10, 10),
        background: Color::rgb(255, 232, 232),
        outline: Color::rgb(190, 25, 35),
        center: Color::rgb(150, 10, 20),
        pupil_scale: None,
    },
    ThemePalette {
        name: "cartoon",
        foreground: Color::rgb(15, 20, 35),
        background: Color::rgb(255, 255, 255),
        outline: Color::rgb(15, 20, 35),
        center: Color::rgb(25, 95, 215),
        pupil_scale: Some(0.34),
    },
    ThemePalette {
        name: "monochrome",
        foreground: Color::rgb(200, 200, 200),
        background: Color::rgb(20, 20, 20),
        outline: Color::rgb(200, 200, 200),
        center: Color::rgb(200, 200, 200),
        pupil_scale: None,
    },
];

pub fn lookup_theme(name: &str) -> Option<&'static ThemePalette> {
    let norm = name.trim().to_ascii_lowercase();
    BUILTIN_THEMES.iter().find(|t| t.name == norm)
}

pub fn next_theme_name(current: &str) -> &'static str {
    let norm = current.trim().to_ascii_lowercase();
    let idx = BUILTIN_THEMES
        .iter()
        .position(|t| t.name == norm)
        .unwrap_or(0);
    BUILTIN_THEMES[(idx + 1) % BUILTIN_THEMES.len()].name
}

/// Normalizes an X11 color name by lowercasing, stripping spaces/hyphens/underscores,
/// and converting `grey` -> `gray`.
fn normalize_color_name(name: &str) -> String {
    let mut s = String::with_capacity(name.len());
    for ch in name.chars() {
        if !ch.is_whitespace() && ch != '-' && ch != '_' {
            s.push(ch.to_ascii_lowercase());
        }
    }
    s.replace("grey", "gray")
}

fn lookup_x11_color(name: &str) -> Option<Color> {
    let norm = normalize_color_name(name);

    // Handle gray0..gray100 dynamically
    if let Some(num_str) = norm.strip_prefix("gray") {
        if !num_str.is_empty() && num_str.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(pct) = num_str.parse::<u32>() {
                if pct <= 100 {
                    let v = ((pct * 255 + 50) / 100) as u8;
                    return Some(Color::rgb(v, v, v));
                }
            }
        }
    }

    X11_COLORS
        .iter()
        .find(|(k, _, _, _)| *k == norm)
        .map(|&(_, r, g, b)| Color::rgb(r, g, b))
}

/// Standard X11 / CSS named colors (`rgb.txt` canonical entries).
const X11_COLORS: &[(&str, u8, u8, u8)] = &[
    ("aliceblue", 240, 248, 255),
    ("antiquewhite", 250, 235, 215),
    ("aqua", 0, 255, 255),
    ("aquamarine", 127, 255, 212),
    ("azure", 240, 255, 255),
    ("beige", 245, 245, 220),
    ("bisque", 255, 228, 196),
    ("black", 0, 0, 0),
    ("blanchedalmond", 255, 235, 205),
    ("blue", 0, 0, 255),
    ("blueviolet", 138, 43, 226),
    ("brown", 165, 42, 42),
    ("burlywood", 222, 184, 135),
    ("cadetblue", 95, 158, 160),
    ("chartreuse", 127, 255, 0),
    ("chocolate", 210, 105, 30),
    ("coral", 255, 127, 80),
    ("cornflowerblue", 100, 149, 237),
    ("cornsilk", 255, 248, 220),
    ("crimson", 220, 20, 60),
    ("cyan", 0, 255, 255),
    ("darkblue", 0, 0, 139),
    ("darkcyan", 0, 139, 139),
    ("darkgoldenrod", 184, 134, 11),
    ("darkgray", 169, 169, 169),
    ("darkgreen", 0, 100, 0),
    ("darkkhaki", 189, 183, 107),
    ("darkmagenta", 139, 0, 139),
    ("darkolivegreen", 85, 107, 47),
    ("darkorange", 255, 140, 0),
    ("darkorchid", 153, 50, 204),
    ("darkred", 139, 0, 0),
    ("darksalmon", 233, 150, 122),
    ("darkseagreen", 143, 188, 143),
    ("darkslateblue", 72, 61, 139),
    ("darkslategray", 47, 79, 79),
    ("darkturquoise", 0, 206, 209),
    ("darkviolet", 148, 0, 211),
    ("deeppink", 255, 20, 147),
    ("deepskyblue", 0, 191, 255),
    ("dimgray", 105, 105, 105),
    ("dodgerblue", 30, 144, 255),
    ("firebrick", 178, 34, 34),
    ("floralwhite", 255, 250, 240),
    ("forestgreen", 34, 139, 34),
    ("fuchsia", 255, 0, 255),
    ("gainsboro", 220, 220, 220),
    ("ghostwhite", 248, 248, 255),
    ("gold", 255, 215, 0),
    ("goldenrod", 218, 165, 32),
    ("gray", 190, 190, 190),
    ("green", 0, 255, 0),
    ("greenyellow", 173, 255, 47),
    ("honeydew", 240, 255, 240),
    ("hotpink", 255, 105, 180),
    ("indianred", 205, 92, 92),
    ("indigo", 75, 0, 130),
    ("ivory", 255, 255, 240),
    ("khaki", 240, 230, 140),
    ("lavender", 230, 230, 250),
    ("lavenderblush", 255, 240, 245),
    ("lawngreen", 124, 252, 0),
    ("lemonchiffon", 255, 250, 205),
    ("lightblue", 173, 216, 230),
    ("lightcoral", 240, 128, 128),
    ("lightcyan", 224, 255, 255),
    ("lightgoldenrodyellow", 250, 250, 210),
    ("lightgray", 211, 211, 211),
    ("lightgreen", 144, 238, 144),
    ("lightpink", 255, 182, 193),
    ("lightsalmon", 255, 160, 122),
    ("lightseagreen", 32, 178, 170),
    ("lightskyblue", 135, 206, 250),
    ("lightslategray", 119, 136, 153),
    ("lightsteelblue", 176, 196, 222),
    ("lightyellow", 255, 255, 224),
    ("lime", 0, 255, 0),
    ("limegreen", 50, 205, 50),
    ("linen", 250, 240, 230),
    ("magenta", 255, 0, 255),
    ("maroon", 176, 48, 96),
    ("mediumaquamarine", 102, 205, 170),
    ("mediumblue", 0, 0, 205),
    ("mediumorchid", 186, 85, 211),
    ("mediumpurple", 147, 112, 219),
    ("mediumseagreen", 60, 179, 113),
    ("mediumslateblue", 123, 104, 238),
    ("mediumspringgreen", 0, 250, 154),
    ("mediumturquoise", 72, 209, 204),
    ("mediumvioletred", 199, 21, 133),
    ("midnightblue", 25, 25, 112),
    ("mintcream", 245, 255, 250),
    ("mistyrose", 255, 228, 225),
    ("moccasin", 255, 228, 181),
    ("navajowhite", 255, 222, 173),
    ("navy", 0, 0, 128),
    ("navyblue", 0, 0, 128),
    ("oldlace", 253, 245, 230),
    ("olive", 128, 128, 0),
    ("olivedrab", 107, 142, 35),
    ("orange", 255, 165, 0),
    ("orangered", 255, 69, 0),
    ("orchid", 218, 112, 214),
    ("palegoldenrod", 238, 232, 170),
    ("palegreen", 152, 251, 152),
    ("paleturquoise", 175, 238, 238),
    ("palevioletred", 219, 112, 147),
    ("papayawhip", 255, 239, 213),
    ("peachpuff", 255, 218, 185),
    ("peru", 205, 133, 63),
    ("pink", 255, 192, 203),
    ("plum", 221, 160, 221),
    ("powderblue", 176, 224, 230),
    ("purple", 160, 32, 240),
    ("rebeccapurple", 102, 51, 153),
    ("red", 255, 0, 0),
    ("rosybrown", 188, 143, 143),
    ("royalblue", 65, 105, 225),
    ("saddlebrown", 139, 69, 19),
    ("salmon", 250, 128, 114),
    ("sandybrown", 244, 164, 96),
    ("seagreen", 46, 139, 87),
    ("seashell", 255, 245, 238),
    ("sienna", 160, 82, 45),
    ("silver", 192, 192, 192),
    ("skyblue", 135, 206, 235),
    ("slateblue", 106, 90, 205),
    ("slategray", 112, 128, 144),
    ("snow", 255, 250, 250),
    ("springgreen", 0, 255, 127),
    ("steelblue", 70, 130, 180),
    ("tan", 210, 180, 140),
    ("teal", 0, 128, 128),
    ("thistle", 216, 191, 216),
    ("tomato", 255, 99, 71),
    ("turquoise", 64, 224, 208),
    ("violet", 238, 130, 238),
    ("wheat", 245, 222, 179),
    ("white", 255, 255, 255),
    ("whitesmoke", 245, 245, 245),
    ("yellow", 255, 255, 0),
    ("yellowgreen", 154, 205, 50),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_named_colors() {
        assert_eq!(Color::parse("red").unwrap(), Color::rgb(255, 0, 0));
        assert_eq!(Color::parse("White").unwrap(), Color::rgb(255, 255, 255));
        assert_eq!(
            Color::parse("light goldenrod yellow").unwrap(),
            Color::rgb(250, 250, 210)
        );
        assert_eq!(Color::parse("SlateBlue").unwrap(), Color::rgb(106, 90, 205));
        assert_eq!(Color::parse("grey50").unwrap(), Color::rgb(128, 128, 128));
        assert_eq!(Color::parse("gray0").unwrap(), Color::rgb(0, 0, 0));
        assert_eq!(Color::parse("grey100").unwrap(), Color::rgb(255, 255, 255));
    }

    #[test]
    fn test_parse_hex_colors() {
        assert_eq!(Color::parse("#f0a").unwrap(), Color::rgb(255, 0, 170));
        assert_eq!(
            Color::parse("#123456").unwrap(),
            Color::rgb(0x12, 0x34, 0x56)
        );
        assert_eq!(
            Color::parse("#12345678").unwrap(),
            Color::rgba(0x12, 0x34, 0x56, 0x78)
        );
        assert_eq!(
            Color::parse("#ffff00008000").unwrap(),
            Color::rgb(0xff, 0x00, 0x80)
        );
    }

    #[test]
    fn test_parse_x11_rgb_spec() {
        assert_eq!(
            Color::parse("rgb:ff/00/80").unwrap(),
            Color::rgb(255, 0, 128)
        );
        assert_eq!(Color::parse("rgb:f/0/f").unwrap(), Color::rgb(255, 0, 255));
        assert_eq!(
            Color::parse("rgb:ffff/0000/0000").unwrap(),
            Color::rgb(255, 0, 0)
        );
    }

    #[test]
    fn test_invalid_colors() {
        assert!(Color::parse("").is_err());
        assert!(Color::parse("not_a_color").is_err());
        assert!(Color::parse("#12345").is_err());
        assert!(Color::parse("rgb:12/34").is_err());
    }
}
