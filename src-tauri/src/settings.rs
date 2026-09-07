use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::logging::redact;
use crate::steam;

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
    std::fs::create_dir_all(&dir).map_err(|e| {
        log::error!("{} could not be created: {e}", redact(&dir));
        format!("could not create {}: {e}", dir.display())
    })?;
    Ok(dir.join("settings.json"))
}

#[tauri::command]
pub fn load_settings(app: tauri::AppHandle) -> Result<Settings, String> {
    let path = settings_file(&app)?;
    let Ok(raw) = std::fs::read_to_string(&path) else {
        // No file yet is the normal first run, not an error.
        return Ok(Settings::default());
    };
    serde_json::from_str(&raw).map_err(|e| {
        log::error!("{} did not parse: {e}", redact(&path));
        format!("{} is not readable: {e}", path.display())
    })
}

#[tauri::command]
pub fn save_settings(app: tauri::AppHandle, settings: Settings) -> Result<Settings, String> {
    let path = settings_file(&app)?;
    let encoded = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, encoded).map_err(|e| {
        log::error!("{} could not be written: {e}", redact(&path));
        format!("could not write {}: {e}", path.display())
    })?;

    // What was saved, not what it was set to. That an override exists is the
    // fact a "no games found" report turns on; the folder someone chose is
    // theirs, and it does not belong in a file destined for a public issue.
    log::info!(
        "settings saved, sampling every {}ms, steam folder {}",
        settings.sample_interval_ms,
        if settings.steam_path_override.trim().is_empty() {
            "auto-detected"
        } else {
            "set by hand"
        }
    );
    Ok(settings)
}

#[tauri::command]
pub fn reset_settings(app: tauri::AppHandle) -> Result<Settings, String> {
    let path = settings_file(&app)?;
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| {
            log::error!("{} could not be deleted: {e}", redact(&path));
            e.to_string()
        })?;
    }
    log::info!("settings file deleted, defaults restored");
    Ok(Settings::default())
}

/// What the inline check of the Steam folder field found.
///
/// An empty field is a valid answer rather than a missing one, so it gets its
/// own arm instead of being flattened into a pass or a failure. The three ways
/// a typed path can be wrong are separate for the same reason: each one needs a
/// different sentence from the UI.
#[derive(Serialize, Debug, PartialEq)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum SteamPathCheck {
    /// Empty field. Auto-detection runs, which is the normal case.
    Automatic,
    /// A folder with a steamapps directory in it. The scan will work.
    Usable,
    /// Nothing at that path.
    Missing,
    /// Something is there, but it is a file.
    NotAFolder,
    /// A real folder that Steam does not live in.
    NoSteamapps,
}

/// Checks the Steam folder field before the scan is ever attempted.
///
/// This has to be here rather than in the webview because the answer is on
/// disk, and telling someone their path is wrong at the moment they type it is
/// kinder than letting them find out two screens later.
#[tauri::command]
pub fn check_steam_path(path: String) -> SteamPathCheck {
    check_steam_folder(&path)
}

fn check_steam_folder(path: &str) -> SteamPathCheck {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return SteamPathCheck::Automatic;
    }

    let target = Path::new(trimmed);
    if !target.exists() {
        return SteamPathCheck::Missing;
    }
    if !target.is_dir() {
        return SteamPathCheck::NotAFolder;
    }
    if steam::holds_steam_library(target) {
        SteamPathCheck::Usable
    } else {
        SteamPathCheck::NoSteamapps
    }
}

#[tauri::command]
pub fn config_path(app: tauri::AppHandle) -> Result<String, String> {
    settings_file(&app).map(|p| p.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn an_empty_field_means_auto_detection_and_is_not_an_error() {
        // The default state of the setting. Reporting it as a problem would put
        // an error under a field almost nobody needs to fill in.
        assert_eq!(check_steam_folder(""), SteamPathCheck::Automatic);
    }

    #[test]
    fn whitespace_alone_is_still_an_empty_field() {
        // candidate_roots trims before it decides whether an override exists,
        // so the check has to agree with it or the two disagree about a space.
        for blank in ["   ", "\t", "\n"] {
            assert_eq!(check_steam_folder(blank), SteamPathCheck::Automatic);
        }
    }

    #[test]
    fn a_path_that_is_not_there_reports_missing() {
        let dir = TempDir::new().expect("temp dir");
        let absent = dir.path().join("no-such-folder");

        assert_eq!(
            check_steam_folder(&absent.display().to_string()),
            SteamPathCheck::Missing
        );
    }

    #[test]
    fn a_file_is_told_apart_from_a_folder() {
        // Pointing at steam.exe instead of the folder holding it is the mistake
        // this arm exists for, and "no steamapps inside it" would be a confusing
        // thing to say about a file.
        let dir = TempDir::new().expect("temp dir");
        let file = dir.path().join("steam.exe");
        fs::write(&file, "not really").expect("write file");

        assert_eq!(
            check_steam_folder(&file.display().to_string()),
            SteamPathCheck::NotAFolder
        );
    }

    #[test]
    fn a_real_folder_without_steamapps_is_not_a_steam_install() {
        let dir = TempDir::new().expect("temp dir");

        assert_eq!(
            check_steam_folder(&dir.path().display().to_string()),
            SteamPathCheck::NoSteamapps
        );
    }

    #[test]
    fn a_folder_holding_steamapps_is_usable() {
        let dir = TempDir::new().expect("temp dir");
        fs::create_dir(dir.path().join("steamapps")).expect("steamapps");

        assert_eq!(
            check_steam_folder(&dir.path().display().to_string()),
            SteamPathCheck::Usable
        );
    }

    #[test]
    fn a_path_wrapped_in_spaces_is_checked_as_the_path_it_names() {
        // Pasting a path out of the address bar drags whitespace along with it.
        let dir = TempDir::new().expect("temp dir");
        fs::create_dir(dir.path().join("steamapps")).expect("steamapps");

        assert_eq!(
            check_steam_folder(&format!("  {}  ", dir.path().display())),
            SteamPathCheck::Usable
        );
    }

    #[test]
    fn the_check_agrees_with_the_scanner_about_the_same_folder() {
        // The point of sharing holds_steam_library: a folder the field calls
        // usable has to be one the scan can actually read, or the inline check
        // is just a second opinion.
        let dir = TempDir::new().expect("temp dir");
        fs::create_dir(dir.path().join("steamapps")).expect("steamapps");
        let path = dir.path().display().to_string();

        assert_eq!(check_steam_folder(&path), SteamPathCheck::Usable);
        assert!(steam::holds_steam_library(dir.path()));
    }

    #[test]
    fn the_serialised_shape_is_a_tagged_status_the_webview_can_switch_on() {
        assert_eq!(
            serde_json::to_string(&SteamPathCheck::NoSteamapps).expect("serialise"),
            r#"{"status":"noSteamapps"}"#
        );
        assert_eq!(
            serde_json::to_string(&SteamPathCheck::Automatic).expect("serialise"),
            r#"{"status":"automatic"}"#
        );
    }
}
