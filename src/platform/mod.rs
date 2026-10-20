use directories::BaseDirs;
use std::fs;
use std::path::PathBuf;

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
