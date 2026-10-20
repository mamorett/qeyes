use qeyes::app::compute_diagnostic_state;
use qeyes::cli::CliArgs;
use qeyes::config::store::{load_settings, save_settings};
use qeyes::config::xrm::XResourceOverrides;
use qeyes::config::Settings;
use qeyes::geometry::{compute_pupil_position, EyesLayout, Point, Size};
use qeyes::render::render_eyes_frame;
use qeyes::ui::menu::ContextMenuState;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_cli_dump_state_with_classic_xeyes_options() {
    let dir = tempdir().unwrap();
    let cfg_path = dir.path().join("test_config.toml");

    let exe = env!("CARGO_BIN_EXE_qeyes");
    let output = Command::new(exe)
        .args([
            "--config",
            cfg_path.to_str().unwrap(),
            "-fg",
            "red",
            "-bg",
            "white",
            "-geometry",
            "300x150+100+100",
            "-rv",
            "+shape",
            "-no-render",
            "--test-pointer",
            "500,80",
            "--dump-state",
        ])
        .output()
        .expect("Failed to execute qeyes binary");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["window_width"], 300);
    assert_eq!(json["window_height"], 150);
    assert_eq!(json["reverse_video"], true);
    assert_eq!(json["shaped"], false);
    assert_eq!(json["antialias"], false);
    // Because -rv swapped fg (red) and bg (white):
    assert_eq!(json["foreground"], "#ffffff");
    assert_eq!(json["background"], "#ff0000");

    // Pointer is at x=500 (to the right of the 300px window), so both pupils look right
    let left_cx = json["left_eye_center"]["x"].as_f64().unwrap();
    let right_cx = json["right_eye_center"]["x"].as_f64().unwrap();
    let left_px = json["left_pupil"]["x"].as_f64().unwrap();
    let right_px = json["right_pupil"]["x"].as_f64().unwrap();
    assert!(left_px > left_cx);
    assert!(right_px > right_cx);
}

#[test]
fn test_cli_save_and_reset_config_persistence() {
    let dir = tempdir().unwrap();
    let cfg_path = dir.path().join("persisted.toml");
    let exe = env!("CARGO_BIN_EXE_qeyes");

    // 1. Run with --save and custom geometry + theme
    let status = Command::new(exe)
        .args([
            "--config",
            cfg_path.to_str().unwrap(),
            "--theme",
            "neon",
            "-geometry",
            "280x160+40+60",
            "--save",
            "--print-config",
        ])
        .output()
        .unwrap();
    assert!(status.status.success());

    let loaded = load_settings(&cfg_path);
    assert_eq!(loaded.window.width, 280);
    assert_eq!(loaded.window.height, 160);
    assert_eq!(loaded.window.x, Some(40));
    assert_eq!(loaded.window.y, Some(60));
    assert_eq!(loaded.appearance.theme, "neon");

    // 2. Run with --reset-config
    let reset_status = Command::new(exe)
        .args([
            "--config",
            cfg_path.to_str().unwrap(),
            "--reset-config",
            "--print-config",
        ])
        .output()
        .unwrap();
    assert!(reset_status.status.success());

    let after_reset = load_settings(&cfg_path);
    assert_eq!(after_reset, Settings::default());
}

#[test]
fn test_golden_render_all_themes_and_states() {
    for theme_name in [
        "classic",
        "dark",
        "neon",
        "solarized",
        "bloodshot",
        "cartoon",
        "monochrome",
    ] {
        let mut settings = Settings::default();
        settings.set_theme(theme_name);

        let layout = EyesLayout::compute(
            Size::new(200.0, 120.0),
            settings.appearance.outline_width,
            settings.appearance.pupil_scale,
        );
        let ptr = Point::new(350.0, 60.0);
        let lp = compute_pupil_position(&layout.left, ptr, true);
        let rp = compute_pupil_position(&layout.right, ptr, true);

        let pixmap = render_eyes_frame(&settings, 200, 120, 1.0, lp, rp, 1.0, None);
        assert_eq!(pixmap.width(), 200);
        assert_eq!(pixmap.height(), 120);

        // Verify pupil pixel matches resolved center color
        let colors = settings.resolved_colors();
        let px = pixmap
            .pixel(lp.x.round() as u32, lp.y.round() as u32)
            .unwrap();
        assert_eq!(px.red(), colors.center.r);
        assert_eq!(px.green(), colors.center.g);
        assert_eq!(px.blue(), colors.center.b);
        assert_eq!(px.alpha(), 255);
    }

    // Verify closed blink frame and context menu rendering do not panic and produce valid PNG
    let settings = Settings::default();
    let mut menu = ContextMenuState::default();
    menu.open(Point::new(10.0, 10.0));
    menu.hovered_index = Some(0);

    let pixmap = render_eyes_frame(
        &settings,
        240,
        160,
        1.0,
        Point::new(60.0, 80.0),
        Point::new(180.0, 80.0),
        0.0,
        Some(&menu),
    );

    let mut png_bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_bytes, pixmap.width(), pixmap.height());
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(pixmap.data()).unwrap();
    }
    assert!(png_bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
}

#[test]
fn test_xrm_and_cli_combined_integration() {
    let dir = tempdir().unwrap();
    let cfg_path = dir.path().join("config.toml");
    let mut initial = Settings::default();
    initial.window.width = 180;
    save_settings(&cfg_path, &initial).unwrap();

    let mut settings = load_settings(&cfg_path);
    let mut xrm = XResourceOverrides::default();
    xrm.parse_lines("Eyes.foreground: darkred\nEyes.outline: gold", "qeyes")
        .unwrap();
    settings.apply_xrm(&xrm);

    let cli = CliArgs::parse_from_raw(["qeyes", "-center", "cyan"]).unwrap();
    settings.apply_cli(&cli).unwrap();

    let state = compute_diagnostic_state(&settings, None, Some("90,50")).unwrap();
    assert_eq!(state.window_width, 180);
    assert_eq!(state.foreground, "#8b0000");
    let resolved = settings.resolved_colors();
    assert_eq!(resolved.outline.to_hex(), "#ffd700");
    assert_eq!(resolved.center.to_hex(), "#00ffff");
}

#[test]
fn test_x11_live_root_pointer_tracking() {
    if std::env::var_os("DISPLAY").is_none() {
        return;
    }

    let dir = tempdir().unwrap();
    let cfg_path = dir.path().join("x11_test.toml");

    let exe = env!("CARGO_BIN_EXE_qeyes");
    let output = Command::new(exe)
        .args([
            "--config",
            cfg_path.to_str().unwrap(),
            "-geometry",
            "150x100+0+0",
            "--dump-state",
        ])
        .output()
        .expect("Failed to execute qeyes --dump-state");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let px = json["pointer"]["x"].as_f64().unwrap();
    let py = json["pointer"]["y"].as_f64().unwrap();
    assert!(px >= 0.0 && py >= 0.0);
}
