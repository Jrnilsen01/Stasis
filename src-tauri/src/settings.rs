use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::Manager;

/// Persisted preferences. Kept deliberately small: every field here is read by
/// something, so there is no setting that looks real but changes nothing.
#[derive(Serialize, Deserialize, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// Overrides Steam auto-detection. Consumed by `scan_steam_games`.
    pub steam_path_override: String,
    /// How often the footprint readout re-samples. Consumed by the poll loop.
    pub sample_interval_ms: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            steam_path_override: String::new(),
            // Two seconds: fast enough to watch a spike, slow enough that the
            // sampler itself is not a meaningful part of what it measures.
            sample_interval_ms: 2000,
        }
    }
}

fn settings_file(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("no config directory available: {e}"))?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    Ok(dir.join("settings.json"))
}

#[tauri::command]
pub fn load_settings(app: tauri::AppHandle) -> Result<Settings, String> {
    let path = settings_file(&app)?;
    let Ok(raw) = std::fs::read_to_string(&path) else {
        // No file yet is the normal first run, not an error.
        return Ok(Settings::default());
    };
    serde_json::from_str(&raw).map_err(|e| format!("{} is not readable: {e}", path.display()))
}

#[tauri::command]
pub fn save_settings(app: tauri::AppHandle, settings: Settings) -> Result<Settings, String> {
    let path = settings_file(&app)?;
    let encoded = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, encoded)
        .map_err(|e| format!("could not write {}: {e}", path.display()))?;
    Ok(settings)
}

#[tauri::command]
pub fn reset_settings(app: tauri::AppHandle) -> Result<Settings, String> {
    let path = settings_file(&app)?;
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(Settings::default())
}

#[tauri::command]
pub fn config_path(app: tauri::AppHandle) -> Result<String, String> {
    settings_file(&app).map(|p| p.display().to_string())
}
