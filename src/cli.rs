use clap::Parser;
use std::path::PathBuf;

/// Modern cross-platform reimplementation of the classic X11 `xeyes`.
///
/// Supports both standard GNU `--long` options and classic X11 single-dash options
/// such as `-geometry WxH+X+Y`, `-fg COLOR`, `-bg COLOR`, `-rv`, `+rv`, `-shape`, `+shape`,
/// `-render`, `-no-render`, and `-xrm RESOURCE`.
#[derive(Debug, Clone, Parser)]
#[command(
    name = "qeyes",
    version,
    about = "A modern cross-platform xeyes implementation",
    after_help = "CLASSIC X11 OPTIONS:\n  \
        -geometry WxH+X+Y      Initial window size and position (e.g. 300x150+100+100, -0-0)\n  \
        -display DISPLAY       X11 display connection string\n  \
        -fg, -foreground CLR   Foreground color (pupils and outline)\n  \
        -bg, -background CLR   Background color (eye interior)\n  \
        -outline CLR           Outline ring color\n  \
        -center CLR            Pupil center color\n  \
        -bw, -borderwidth N    Eye outline thickness in pixels\n  \
        -rv / +rv              Enable / disable reverse video\n  \
        -shape / +shape        Enable / disable shaped window\n  \
        -render / -no-render   Enable / disable anti-aliased rendering\n  \
        -xrm RESOURCE          X11 resource override (e.g. 'Eyes.foreground: red')"
)]
pub struct CliArgs {
    /// Initial window geometry (`WxH+X+Y`, e.g. `300x150+100+100` or `-0-0`)
    #[arg(long = "geometry", value_name = "WxH+X+Y", allow_hyphen_values = true)]
    pub geometry: Option<String>,

    /// X11 display to connect to (e.g. `:0`)
    #[arg(long = "display", value_name = "DISPLAY")]
    pub display: Option<String>,

    /// Foreground color (pupils and outline)
    #[arg(long = "foreground", visible_alias = "fg", value_name = "COLOR")]
    pub foreground: Option<String>,

    /// Background color (eye interior)
    #[arg(long = "background", visible_alias = "bg", value_name = "COLOR")]
    pub background: Option<String>,

    /// Eye outline color
    #[arg(long = "outline", value_name = "COLOR")]
    pub outline: Option<String>,

    /// Eye pupil (center) color
    #[arg(long = "center", value_name = "COLOR")]
    pub center: Option<String>,

    /// Outline border width in pixels
    #[arg(long = "borderwidth", visible_alias = "bw", value_name = "PIXELS")]
    pub borderwidth: Option<f64>,

    /// Enable reverse video (swap foreground and background)
    #[arg(long = "rv")]
    pub rv: bool,

    /// Disable reverse video (`+rv`)
    #[arg(long = "no-rv")]
    pub no_rv: bool,

    /// Enable shaped/transparent window (`-shape`)
    #[arg(long = "shape")]
    pub shape: bool,

    /// Disable shaped/transparent window (`+shape`)
    #[arg(long = "no-shape")]
    pub no_shape: bool,

    /// Enable anti-aliased rendering (`-render`)
    #[arg(long = "render")]
    pub render: bool,

    /// Disable anti-aliased rendering (`+render` or `-no-render`)
    #[arg(long = "no-render")]
    pub no_render: bool,

    /// X11 backing store type (accepted for xeyes compatibility and ignored)
    #[arg(long = "backing", value_name = "TYPE", hide = true)]
    pub backing: Option<String>,

    /// X resource string override (can be specified multiple times, e.g. `-xrm 'Eyes.foreground: red'`)
    #[arg(long = "xrm", value_name = "RESOURCE")]
    pub xrm: Vec<String>,

    /// Application instance name (`WM_CLASS` instance)
    #[arg(long = "name", value_name = "NAME")]
    pub name: Option<String>,

    /// Window title
    #[arg(long = "title", value_name = "TITLE")]
    pub title: Option<String>,

    /// Built-in visual theme (`classic`, `dark`, `nord`, `neon`, `solarized`, `bloodshot`, `cartoon`, `monochrome`)
    #[arg(long = "theme", value_name = "THEME")]
    pub theme: Option<String>,

    /// Keep window always on top of other windows
    #[arg(long = "always-on-top")]
    pub always_on_top: bool,

    /// Make window click-through (mouse events pass through to underlying windows)
    #[arg(long = "click-through")]
    pub click_through: bool,

    /// Show standard OS window decorations (title bar and border)
    #[arg(long = "decorations")]
    pub decorations: bool,

    /// Lock aspect ratio when resizing
    #[arg(long = "lock-aspect")]
    pub lock_aspect: bool,

    /// Enable periodic natural eye blinking
    #[arg(long = "blink")]
    pub blink: bool,

    /// Enable smooth pupil tracking interpolation
    #[arg(long = "smooth")]
    pub smooth: bool,

    /// Enable system tray mode
    #[arg(long = "tray")]
    pub tray: bool,

    /// Disable system tray mode
    #[arg(long = "no-tray")]
    pub no_tray: bool,

    /// Configuration profile name (stores settings in `config.<NAME>.toml`)
    #[arg(long = "profile", value_name = "NAME")]
    pub profile: Option<String>,

    /// Explicit path to configuration file
    #[arg(long = "config", value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Persist CLI overrides (including `-geometry`) to the configuration file
    #[arg(long = "save")]
    pub save: bool,

    /// Reset configuration file to built-in defaults
    #[arg(long = "reset-config")]
    pub reset_config: bool,

    /// Print the effective TOML configuration to stdout and exit
    #[arg(long = "print-config")]
    pub print_config: bool,

    /// Compute one frame of state, print JSON diagnostics to stdout, and exit (used by tests)
    #[arg(long = "dump-state")]
    pub dump_state: bool,

    /// Simulated pointer coordinates `X,Y` for `--dump-state` headless verification
    #[arg(
        long = "test-pointer",
        value_name = "X,Y",
        allow_hyphen_values = true,
        hide = true
    )]
    pub test_pointer: Option<String>,

    /// Force X11 Shape extension mask even when a compositor is active
    #[arg(long = "force-xshape")]
    pub force_xshape: bool,

    /// Logging verbosity (`error`, `warn`, `info`, `debug`, `trace`)
    #[arg(long = "log", value_name = "LEVEL")]
    pub log: Option<String>,
}

impl CliArgs {
    /// Normalizes legacy X11 single-dash/plus flags and parses arguments using `clap`.
    pub fn parse_env() -> Self {
        let normalized = normalize_args(std::env::args());
        Self::parse_from(normalized)
    }

    /// Parses from an arbitrary iterator of strings (for unit and integration tests).
    pub fn parse_from_raw<I, T>(raw_args: I) -> Result<Self, clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        let normalized = normalize_args(raw_args.into_iter().map(Into::into));
        Self::try_parse_from(normalized)
    }
}

/// Rewrites classic X11 single-dash options (`-fg`, `-geometry`, `+shape`, `-rv`, etc.)
/// into canonical GNU `--` flags accepted by `clap`.
pub fn normalize_args<I>(args: I) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    let raw: Vec<String> = args.into_iter().collect();
    let mut out = Vec::with_capacity(raw.len());
    let mut iter = raw.into_iter().enumerate();

    while let Some((idx, arg)) = iter.next() {
        if idx == 0 {
            out.push(arg);
            continue;
        }

        if arg == "--" {
            out.push(arg);
            for (_, rest) in iter {
                out.push(rest);
            }
            break;
        }

        match arg.as_str() {
            // Plus-prefixed X11 toggles
            "+rv" => out.push("--no-rv".to_string()),
            "+shape" => out.push("--no-shape".to_string()),
            "+render" => out.push("--no-render".to_string()),

            // Single-dash X11 boolean flags
            "-rv" => out.push("--rv".to_string()),
            "-shape" => out.push("--shape".to_string()),
            "-render" => out.push("--render".to_string()),
            "-no-render" | "-norender" => out.push("--no-render".to_string()),
            "-version" => out.push("--version".to_string()),
            "-help" => out.push("--help".to_string()),

            // Single-dash X11 value flags
            "-geometry" | "--geometry" => {
                if let Some((_, val)) = iter.next() {
                    out.push(format!("--geometry={val}"));
                } else {
                    out.push("--geometry".to_string());
                }
            }
            "-display" => out.push("--display".to_string()),
            "-fg" | "-foreground" => out.push("--foreground".to_string()),
            "-bg" | "-background" => out.push("--background".to_string()),
            "-outline" => out.push("--outline".to_string()),
            "-center" => out.push("--center".to_string()),
            "-bw" | "-borderwidth" => out.push("--borderwidth".to_string()),
            "-backing" => out.push("--backing".to_string()),
            "-xrm" => out.push("--xrm".to_string()),
            "-name" => out.push("--name".to_string()),
            "-title" => out.push("--title".to_string()),

            other => {
                // Ignore macOS Process Serial Number argument (-psn_...) passed by Finder/LaunchServices
                if other.starts_with("-psn") {
                    continue;
                }

                // Also support `-geometry=WxH+X+Y`
                if let Some(val) = other.strip_prefix("-geometry=") {
                    out.push(format!("--geometry={val}"));
                } else {
                    out.push(other.to_string());
                }
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classic_xeyes_cli_flags() {
        let args = CliArgs::parse_from_raw([
            "qeyes",
            "-fg",
            "red",
            "-bg",
            "white",
            "-outline",
            "blue",
            "-center",
            "green",
            "-geometry",
            "300x150-0-0",
            "-bw",
            "4",
            "-rv",
            "+shape",
            "-no-render",
            "-backing",
            "WhenMapped",
            "-xrm",
            "Eyes.foreground: yellow",
            "-name",
            "myeyes",
            "-title",
            "My Eyes",
        ])
        .unwrap();

        assert_eq!(args.foreground.as_deref(), Some("red"));
        assert_eq!(args.background.as_deref(), Some("white"));
        assert_eq!(args.outline.as_deref(), Some("blue"));
        assert_eq!(args.center.as_deref(), Some("green"));
        assert_eq!(args.geometry.as_deref(), Some("300x150-0-0"));
        assert_eq!(args.borderwidth, Some(4.0));
        assert!(args.rv);
        assert!(args.no_shape);
        assert!(args.no_render);
        assert_eq!(args.backing.as_deref(), Some("WhenMapped"));
        assert_eq!(args.xrm, vec!["Eyes.foreground: yellow"]);
        assert_eq!(args.name.as_deref(), Some("myeyes"));
        assert_eq!(args.title.as_deref(), Some("My Eyes"));
    }

    #[test]
    fn test_plus_rv_and_plus_render() {
        let args = CliArgs::parse_from_raw(["qeyes", "+rv", "+render", "-shape"]).unwrap();
        assert!(args.no_rv);
        assert!(args.no_render);
        assert!(args.shape);
    }

    #[test]
    fn test_macos_psn_arg_ignored() {
        let args = CliArgs::parse_from_raw(["qeyes", "-psn_0_1234567"]).unwrap();
        assert_eq!(args.foreground, None);
    }
}
