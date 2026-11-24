use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppConfig {
    pub is_light_mode: bool,
    pub bg_mode: u8,             // 0=checkerboard, 1=white, 2=black, 3=slate
    pub export_scale_preset: u8, // 1, 2, 4, 8
    pub export_bg_mode: u8,      // 0=transparent, 1=white, 2=black
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            is_light_mode: true, // Default: Light mode
            bg_mode: 1,          // Default: White background
            export_scale_preset: 1,
            export_bg_mode: 0,
        }
    }
}

pub fn get_config_path() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        if let Ok(app_data) = std::env::var("APPDATA") {
            let dir = PathBuf::from(app_data).join("SvagView");
            return Some(dir.join("settings.json"));
        }
    }

    #[cfg(unix)]
    {
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            let dir = PathBuf::from(xdg).join("svagview");
            return Some(dir.join("settings.json"));
        }
        if let Ok(home) = std::env::var("HOME") {
            let dir = PathBuf::from(home).join(".config").join("svagview");
            return Some(dir.join("settings.json"));
        }
    }

    // Generic fallback: current directory
    if let Ok(cwd) = std::env::current_dir() {
        return Some(cwd.join(".svagview_settings.json"));
    }

    None
}

impl AppConfig {
    pub fn load() -> Self {
        if let Some(path) = get_config_path() {
            if path.is_file() {
                if let Ok(data) = std::fs::read_to_string(&path) {
                    if let Ok(cfg) = serde_json::from_str::<AppConfig>(&data) {
                        return cfg;
                    }
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), String> {
        let Some(path) = get_config_path() else {
            return Err("Cannot determine configuration directory".to_string());
        };

        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Serialization error: {}", e))?;

        std::fs::write(&path, json)
            .map_err(|e| format!("Failed to write config file '{}': {}", path.display(), e))?;

        Ok(())
    }
}
