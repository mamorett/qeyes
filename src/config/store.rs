use crate::config::Settings;
use directories::ProjectDirs;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Returns the default configuration file path for `qeyes`, optionally scoped by `profile`.
///
/// - Linux: `$XDG_CONFIG_HOME/qeyes/config.toml` (or `config.<profile>.toml`)
/// - macOS: `~/Library/Application Support/qeyes/config.toml`
/// - Windows: `%APPDATA%\qeyes\config.toml`
pub fn resolve_config_path(explicit_path: Option<&Path>, profile: Option<&str>) -> PathBuf {
    if let Some(p) = explicit_path {
        return p.to_path_buf();
    }

    let base_dir = ProjectDirs::from("", "", "qeyes")
        .map(|d| d.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    let filename = match profile {
        Some(prof) if !prof.trim().is_empty() => {
            let sanitized: String = prof
                .trim()
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect();
            format!("config.{sanitized}.toml")
        }
        _ => "config.toml".to_string(),
    };

    base_dir.join(filename)
}

/// Loads `Settings` from `path`.
///
/// - If `path` does not exist, returns `Settings::default()`.
/// - If `path` contains invalid/corrupt TOML, renames it to `<path>.bak`, logs a warning,
///   and returns `Settings::default()`. Never panics or crashes on bad config.
pub fn load_settings(path: &Path) -> Settings {
    if !path.exists() {
        return Settings::default();
    }

    let raw = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Failed to read config file {}: {e}", path.display());
            return Settings::default();
        }
    };

    match toml::from_str::<Settings>(&raw) {
        Ok(mut settings) => {
            settings.migrate_and_validate();
            settings
        }
        Err(e) => {
            let bak_path = path.with_extension("toml.bak");
            tracing::warn!(
                "Corrupt config file {} ({e}); backing up to {} and using defaults",
                path.display(),
                bak_path.display()
            );
            let _ = fs::rename(path, &bak_path);
            Settings::default()
        }
    }
}

/// Saves `Settings` atomically to `path` (writes `<path>.tmp`, flushes/syncs, then renames).
pub fn save_settings(path: &Path, settings: &Settings) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "Failed to create config directory {}: {e}",
                parent.display()
            )
        })?;
    }

    let serialized = toml::to_string_pretty(settings)
        .map_err(|e| format!("Failed to serialize settings to TOML: {e}"))?;

    let tmp_path = path.with_extension("toml.tmp");
    {
        let mut file = fs::File::create(&tmp_path)
            .map_err(|e| format!("Failed to create temp config {}: {e}", tmp_path.display()))?;
        file.write_all(serialized.as_bytes())
            .map_err(|e| format!("Failed to write temp config {}: {e}", tmp_path.display()))?;
        let _ = file.sync_all();
    }

    fs::rename(&tmp_path, path).map_err(|e| {
        format!(
            "Failed to atomically rename {} to {}: {e}",
            tmp_path.display(),
            path.display()
        )
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_save_and_load_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let mut s = Settings::default();
        s.window.width = 320;
        s.window.height = 180;
        s.window.x = Some(45);
        s.window.y = Some(90);
        s.appearance.theme = "neon".to_string();
        s.behavior.blink = true;

        save_settings(&path, &s).unwrap();
        let loaded = load_settings(&path);
        assert_eq!(loaded, s);
    }

    #[test]
    fn test_corrupt_config_creates_bak_and_returns_defaults() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "this is [[[ not valid toml !!!").unwrap();

        let loaded = load_settings(&path);
        assert_eq!(loaded, Settings::default());
        assert!(!path.exists());
        assert!(dir.path().join("config.toml.bak").exists());
    }

    #[test]
    fn test_profile_path_sanitization() {
        let p = resolve_config_path(None, Some("work/left eye"));
        assert!(p
            .file_name()
            .unwrap()
            .to_string_lossy()
            .eq("config.work_left_eye.toml"));
    }
}
