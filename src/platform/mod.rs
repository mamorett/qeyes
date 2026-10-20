use directories::BaseDirs;
use std::fs;
use std::path::PathBuf;

/// Ensures user desktop integration on Linux (~/.local/share/applications and icons)
/// so that KDE Plasma and other desktop docks/panels can display the application icon
/// and allow "Pin to Task Manager" / "Stick to Dock".
pub fn ensure_desktop_integration() {
    #[cfg(target_os = "linux")]
    {
        let Some(base) = BaseDirs::new() else {
            return;
        };
        let data_dir = base.data_dir(); // ~/.local/share

        // 1. Install icons
        let icon_sizes: &[(&str, &[u8])] = &[
            ("16x16", include_bytes!("../../assets/qeyes-16.png")),
            ("32x32", include_bytes!("../../assets/qeyes-32.png")),
            ("48x48", include_bytes!("../../assets/qeyes-48.png")),
            ("64x64", include_bytes!("../../assets/qeyes-64.png")),
            ("128x128", include_bytes!("../../assets/qeyes-128.png")),
            ("256x256", include_bytes!("../../assets/qeyes-256.png")),
        ];

        for (size_name, png_data) in icon_sizes {
            let dir = data_dir.join("icons/hicolor").join(size_name).join("apps");
            if fs::create_dir_all(&dir).is_ok() {
                let _ = fs::write(dir.join("qeyes.png"), png_data);
            }
        }

        let pixmaps_dir = data_dir.join("pixmaps");
        if fs::create_dir_all(&pixmaps_dir).is_ok() {
            let _ = fs::write(
                pixmaps_dir.join("qeyes.png"),
                include_bytes!("../../assets/qeyes-256.png"),
            );
        }

        let scalable_dir = data_dir.join("icons/hicolor/scalable/apps");
        if fs::create_dir_all(&scalable_dir).is_ok() {
            let _ = fs::write(
                scalable_dir.join("qeyes.svg"),
                include_bytes!("../../assets/qeyes.svg"),
            );
        }

        // 2. Install desktop entry if missing
        let apps_dir = data_dir.join("applications");
        let desktop_file = apps_dir.join("qeyes.desktop");
        let exe_path = std::env::current_exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "qeyes".to_string());

        let desktop_content = format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Version=1.0\n\
             Name=qeyes\n\
             GenericName=Desktop Eyes\n\
             Comment=A modern cross-platform xeyes whose pupils follow your mouse pointer\n\
             Exec=\"{exe_path}\"\n\
             Icon=qeyes\n\
             Terminal=false\n\
             Categories=Utility;Amusement;\n\
             Keywords=xeyes;eyes;mouse;cursor;pointer;x11;\n\
             StartupWMClass=qeyes\n\
             StartupNotify=false\n"
        );

        if !desktop_file.exists() && fs::create_dir_all(&apps_dir).is_ok() {
            let _ = fs::write(&desktop_file, desktop_content);
        }
    }
}

/// Checks if the current Linux session is running under a Wayland compositor (`WAYLAND_DISPLAY` set)
/// and logs a one-time notice explaining XWayland global pointer behavior.
pub fn check_wayland_session_notice() {
    #[cfg(target_os = "linux")]
    {
        if std::env::var_os("WAYLAND_DISPLAY").is_some() {
            tracing::info!(
                "Wayland session detected. qeyes is using the X11/XWayland backend so it can query \
                 the root pointer. Note: under Wayland, global pointer coordinates update when the \
                 cursor moves over XWayland windows or the desktop root."
            );
        }
    }
}

/// Configures or removes the user-level autostart entry (`Run at Login`) for `qeyes`.
pub fn set_autostart(enabled: bool) -> Result<(), String> {
    let exe_path = std::env::current_exe()
        .map_err(|e| format!("Failed to determine current executable path: {e}"))?;
    let exe_str = exe_path.to_string_lossy();

    #[cfg(target_os = "linux")]
    {
        let base = BaseDirs::new().ok_or("Could not resolve home directory")?;
        let autostart_dir = base.config_dir().join("autostart");
        let desktop_file = autostart_dir.join("qeyes.desktop");
        if enabled {
            fs::create_dir_all(&autostart_dir)
                .map_err(|e| format!("Failed to create autostart dir: {e}"))?;
            let content = format!(
                "[Desktop Entry]\n\
                 Type=Application\n\
                 Name=qeyes\n\
                 Comment=Modern cross-platform xeyes\n\
                 Exec=\"{exe_str}\"\n\
                 Icon=qeyes\n\
                 Terminal=false\n\
                 Categories=Utility;Amusement;\n\
                 StartupNotify=false\n"
            );
            fs::write(&desktop_file, content)
                .map_err(|e| format!("Failed to write {}: {e}", desktop_file.display()))?;
        } else if desktop_file.exists() {
            let _ = fs::remove_file(&desktop_file);
        }
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        let base = BaseDirs::new().ok_or("Could not resolve home directory")?;
        let agents_dir = base.home_dir().join("Library/LaunchAgents");
        let plist_file = agents_dir.join("com.github.qeyes.plist");
        if enabled {
            fs::create_dir_all(&agents_dir)
                .map_err(|e| format!("Failed to create LaunchAgents dir: {e}"))?;
            let content = format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                 <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
                 <plist version=\"1.0\">\n\
                 <dict>\n\
                     <key>Label</key>\n\
                     <string>com.github.qeyes</string>\n\
                     <key>ProgramArguments</key>\n\
                     <array>\n\
                         <string>{exe_str}</string>\n\
                     </array>\n\
                     <key>RunAtLoad</key>\n\
                     <true/>\n\
                 </dict>\n\
                 </plist>\n"
            );
            fs::write(&plist_file, content)
                .map_err(|e| format!("Failed to write {}: {e}", plist_file.display()))?;
        } else if plist_file.exists() {
            let _ = fs::remove_file(&plist_file);
        }
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .ok_or("APPDATA environment variable not set")?;
        let startup_dir = appdata.join(r"Microsoft\Windows\Start Menu\Programs\Startup");
        let cmd_file = startup_dir.join("qeyes.cmd");
        if enabled {
            fs::create_dir_all(&startup_dir)
                .map_err(|e| format!("Failed to create Startup dir: {e}"))?;
            let content = format!("@echo off\r\nstart \"\" \"{exe_str}\"\r\n");
            fs::write(&cmd_file, content)
                .map_err(|e| format!("Failed to write {}: {e}", cmd_file.display()))?;
        } else if cmd_file.exists() {
            let _ = fs::remove_file(&cmd_file);
        }
        return Ok(());
    }

    #[allow(unreachable_code)]
    {
        let _ = (enabled, exe_str, PathBuf::new(), BaseDirs::new);
        Ok(())
    }
}
