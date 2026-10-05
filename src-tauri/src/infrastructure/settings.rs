use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application settings persisted to disk.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AppSettings {
    pub auto_submit: bool,
}

/// Returns the configuration file path (%APPDATA%\ff14-quickauth\settings.json).
pub fn get_settings_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let app_data = std::env::var("APPDATA").ok()?;
        Some(PathBuf::from(app_data).join("ff14-quickauth").join("settings.json"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var("HOME").ok()?;
        Some(PathBuf::from(home).join(".config").join("ff14-quickauth").join("settings.json"))
    }
}

/// Loads settings from disk, falling back to default if not found or corrupted.
pub fn load_settings() -> AppSettings {
    if let Some(path) = get_settings_path() {
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(settings) = serde_json::from_str::<AppSettings>(&content) {
                    return settings;
                }
            }
        }
    }
    AppSettings::default()
}

/// Saves settings to disk (%APPDATA%\ff14-quickauth\settings.json).
pub fn save_settings(settings: &AppSettings) -> Result<(), String> {
    let path = get_settings_path().ok_or_else(|| "設定ディレクトリの取得に失敗しました".to_string())?;
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("設定フォルダの作成に失敗しました: {e}"))?;
        }
    }
    let json = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("設定のシリアライズに失敗しました: {e}"))?;
    std::fs::write(&path, json)
        .map_err(|e| format!("設定ファイルの書き込みに失敗しました: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_has_auto_submit_false() {
        let settings = AppSettings::default();
        assert!(!settings.auto_submit);
    }

    #[test]
    fn serialization_roundtrip() {
        let settings = AppSettings { auto_submit: true };
        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: AppSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(settings, deserialized);
    }
}
