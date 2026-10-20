use crate::geometry::XGeometry;
use crate::render::theme::Color;

/// Parsed X11 resource overrides from `RESOURCE_MANAGER`, `~/.Xresources`, or `-xrm`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct XResourceOverrides {
    pub foreground: Option<Color>,
    pub background: Option<Color>,
    pub outline: Option<Color>,
    pub center: Option<Color>,
    pub reverse_video: Option<bool>,
    pub shaped: Option<bool>,
    pub render: Option<bool>,
    pub geometry: Option<XGeometry>,
    pub border_width: Option<f64>,
    pub title: Option<String>,
}

impl XResourceOverrides {
    /// Parses one or more lines of X11 resource definitions (e.g. `Eyes.foreground: red`).
    ///
    /// Supports prefixes `Eyes`, `eyes`, `qeyes`, `QEyes`, or wildcard `*`.
    pub fn parse_lines(&mut self, text: &str, app_name: &str) -> Result<(), String> {
        let custom_prefix = app_name.trim().to_ascii_lowercase();

        for raw_line in text.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('!') || line.starts_with('#') {
                continue;
            }

            let Some((raw_key, raw_val)) = line.split_once(':') else {
                return Err(format!("Invalid X resource syntax (missing ':'): '{line}'"));
            };

            let key = raw_key.trim();
            let val = raw_val.trim();

            if let Some(prop) = Self::extract_matching_property(key, &custom_prefix) {
                self.apply_property(prop, val)?;
            }
        }
        Ok(())
    }

    fn extract_matching_property<'a>(key: &'a str, custom_prefix: &str) -> Option<&'a str> {
        if let Some(rest) = key.strip_prefix('*') {
            let prop = rest.trim_start_matches(['.', '*']);
            if !prop.contains(['.', '*']) {
                return Some(prop);
            }
        }

        // Split on first `.` or `*`
        let sep_idx = key.find(['.', '*'])?;
        let prefix = key[..sep_idx].trim().to_ascii_lowercase();
        let prop = key[sep_idx + 1..].trim_start_matches(['.', '*']);

        if prefix == "eyes" || prefix == "qeyes" || prefix == custom_prefix {
            Some(prop)
        } else {
            None
        }
    }

    fn apply_property(&mut self, prop: &str, val: &str) -> Result<(), String> {
        let prop_lower = prop.to_ascii_lowercase();
        match prop_lower.as_str() {
            "foreground" | "fg" => {
                self.foreground = Some(Color::parse(val)?);
            }
            "background" | "bg" => {
                self.background = Some(Color::parse(val)?);
            }
            "outline" => {
                self.outline = Some(Color::parse(val)?);
            }
            "center" => {
                self.center = Some(Color::parse(val)?);
            }
            "reversevideo" | "rv" => {
                self.reverse_video = Some(parse_xrm_bool(val)?);
            }
            "shapewindow" | "shape" => {
                self.shaped = Some(parse_xrm_bool(val)?);
            }
            "render" => {
                self.render = Some(parse_xrm_bool(val)?);
            }
            "geometry" => {
                self.geometry = Some(XGeometry::parse(val)?);
            }
            "borderwidth" | "bw" => {
                let bw: f64 = val
                    .parse()
                    .map_err(|_| format!("Invalid borderWidth '{val}'"))?;
                self.border_width = Some(bw);
            }
            "title" => {
                self.title = Some(val.to_string());
            }
            _ => {
                // Unknown resource keys are ignored per Xrm conventions
            }
        }
        Ok(())
    }

    /// Loads system/user X resources when available (`RESOURCE_MANAGER` on X11, or `~/.Xresources`).
    pub fn load_from_environment(app_name: &str) -> Self {
        let mut overrides = Self::default();

        // 1. Check ~/.Xresources if present
        if let Some(base) = directories::BaseDirs::new() {
            let xres_path = base.home_dir().join(".Xresources");
            if let Ok(contents) = std::fs::read_to_string(xres_path) {
                let _ = overrides.parse_lines(&contents, app_name);
            }
        }

        // 2. Query X11 root window RESOURCE_MANAGER property on Linux
        #[cfg(target_os = "linux")]
        {
            if let Some(rm_str) = query_x11_resource_manager() {
                let _ = overrides.parse_lines(&rm_str, app_name);
            }
        }

        overrides
    }
}

fn parse_xrm_bool(val: &str) -> Result<bool, String> {
    match val.trim().to_ascii_lowercase().as_str() {
        "true" | "on" | "yes" | "1" => Ok(true),
        "false" | "off" | "no" | "0" => Ok(false),
        _ => Err(format!("Invalid boolean X resource value '{val}'")),
    }
}

#[cfg(target_os = "linux")]
fn query_x11_resource_manager() -> Option<String> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt};

    let (conn, screen_num) = x11rb::connect(None).ok()?;
    let screen = &conn.setup().roots[screen_num];
    let reply = conn
        .get_property(
            false,
            screen.root,
            AtomEnum::RESOURCE_MANAGER,
            AtomEnum::STRING,
            0,
            100_000,
        )
        .ok()?
        .reply()
        .ok()?;

    if reply.value.is_empty() {
        None
    } else {
        String::from_utf8(reply.value).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_xrm_lines() {
        let mut xrm = XResourceOverrides::default();
        let input = r#"
            ! Comment line
            Eyes.foreground: red
            Eyes.background: #102030
            qeyes*outline: green
            *center: blue
            Eyes.reverseVideo: true
            Eyes.shapeWindow: off
            Eyes.render: false
            Eyes.geometry: 250x120+10+20
            OtherApp.foreground: yellow
        "#;
        xrm.parse_lines(input, "qeyes").unwrap();

        assert_eq!(xrm.foreground, Some(Color::rgb(255, 0, 0)));
        assert_eq!(xrm.background, Some(Color::rgb(0x10, 0x20, 0x30)));
        assert_eq!(xrm.outline, Some(Color::rgb(0, 255, 0)));
        assert_eq!(xrm.center, Some(Color::rgb(0, 0, 255)));
        assert_eq!(xrm.reverse_video, Some(true));
        assert_eq!(xrm.shaped, Some(false));
        assert_eq!(xrm.render, Some(false));
        assert_eq!(xrm.geometry.unwrap().width, Some(250));
    }

    #[test]
    fn test_custom_app_name_xrm() {
        let mut xrm = XResourceOverrides::default();
        xrm.parse_lines("myeyes.foreground: purple", "myeyes")
            .unwrap();
        assert_eq!(xrm.foreground, Some(Color::rgb(160, 32, 240)));
    }
}
