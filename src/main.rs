#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use qeyes::app::{compute_diagnostic_state, QEyesApp};
use qeyes::cli::CliArgs;
use qeyes::config::store::{load_settings, resolve_config_path, save_settings};
use qeyes::config::xrm::XResourceOverrides;
use qeyes::config::Settings;
use qeyes::geometry::GeometryOffset;
use qeyes::platform::check_wayland_session_notice;
use std::process::ExitCode;
use tracing_subscriber::EnvFilter;
use winit::event_loop::EventLoop;

fn main() -> ExitCode {
    #[cfg(target_os = "windows")]
    unsafe {
        use windows_sys::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
    }

    let cli = CliArgs::parse_env();

    let log_level = cli.log.as_deref().unwrap_or("warn");
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();

    if cli.backing.is_some() {
        tracing::debug!("Ignoring legacy -backing option (X11 backing store is obsolete)");
    }

    if let Some(ref disp) = cli.display {
        #[cfg(target_os = "linux")]
        {
            std::env::set_var("DISPLAY", disp);
        }
        #[cfg(not(target_os = "linux"))]
        {
            tracing::warn!("-display '{disp}' is only used on X11/Linux; ignoring on this OS");
        }
    }

    let config_path = resolve_config_path(cli.config.as_deref(), cli.profile.as_deref());

    if cli.reset_config {
        let defaults = Settings::default();
        if let Err(e) = save_settings(&config_path, &defaults) {
            eprintln!(
                "qeyes: failed to reset config {}: {e}",
                config_path.display()
            );
            return ExitCode::FAILURE;
        }
    }

    let mut settings = load_settings(&config_path);

    // Apply X resources (environment + explicit -xrm flags)
    let app_name = cli
        .name
        .as_deref()
        .unwrap_or(&settings.window.app_name)
        .to_string();
    let mut xrm = XResourceOverrides::load_from_environment(&app_name);
    for rule in &cli.xrm {
        if let Err(e) = xrm.parse_lines(rule, &app_name) {
            eprintln!("qeyes: invalid -xrm resource '{rule}': {e}");
            return ExitCode::FAILURE;
        }
    }
    let xrm_geom = settings.apply_xrm(&xrm);

    let cli_geom = match settings.apply_cli(&cli) {
        Ok(g) => g.or(xrm_geom),
        Err(e) => {
            eprintln!("qeyes: {e}");
            return ExitCode::FAILURE;
        }
    };

    if cli.save {
        if let Some(g) = cli_geom {
            if let (Some(GeometryOffset::Positive(x)), Some(GeometryOffset::Positive(y))) =
                (g.x, g.y)
            {
                settings.window.x = Some(x);
                settings.window.y = Some(y);
            }
        }
        if let Err(e) = save_settings(&config_path, &settings) {
            eprintln!(
                "qeyes: failed to save config {}: {e}",
                config_path.display()
            );
            return ExitCode::FAILURE;
        }
    }

    if cli.print_config {
        match toml::to_string_pretty(&settings) {
            Ok(s) => {
                print!("{s}");
                return ExitCode::SUCCESS;
            }
            Err(e) => {
                eprintln!("qeyes: failed to serialize config: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    if cli.dump_state {
        match compute_diagnostic_state(
            &settings,
            cli.display.as_deref(),
            cli.test_pointer.as_deref(),
        ) {
            Ok(state) => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&state).unwrap_or_default()
                );
                return ExitCode::SUCCESS;
            }
            Err(e) => {
                eprintln!("qeyes: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    check_wayland_session_notice();
    qeyes::platform::ensure_desktop_integration();

    let event_loop = match create_platform_event_loop() {
        Ok(el) => el,
        Err(e) => {
            eprintln!("qeyes: failed to initialize display event loop: {e}");
            return ExitCode::FAILURE;
        }
    };

    let mut app = QEyesApp::new(
        settings,
        config_path,
        cli_geom,
        cli.display,
        cli.force_xshape,
    );

    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("qeyes: event loop error: {e}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn create_platform_event_loop() -> Result<EventLoop<()>, winit::error::EventLoopError> {
    #[cfg(target_os = "linux")]
    {
        use winit::platform::x11::EventLoopBuilderExtX11;
        let mut builder = EventLoop::builder();
        builder.with_x11();
        if let Ok(el) = builder.build() {
            return Ok(el);
        }
    }
    EventLoop::new()
}
