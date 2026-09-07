mod footprint;
mod modules;
mod settings;
mod steam;

use std::sync::Mutex;

use footprint::FootprintState;

/// Opens a path in the OS file manager.
///
/// Implemented here rather than through a plugin so the empty states can offer a
/// real action: every "open folder" affordance in the UI resolves to this.
#[tauri::command]
fn reveal_path(path: String) -> Result<(), String> {
    let target = std::path::PathBuf::from(&path);
    if !target.exists() {
        std::fs::create_dir_all(&target).map_err(|e| format!("could not create {path}: {e}"))?;
    }

    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("explorer").arg(&target).spawn();
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(&target).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let result = std::process::Command::new("xdg-open").arg(&target).spawn();

    result.map(|_| ()).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(FootprintState::new()))
        .invoke_handler(tauri::generate_handler![
            footprint::read_footprint,
            steam::scan_steam_games,
            modules::list_modules,
            modules::modules_dir,
            settings::load_settings,
            settings::save_settings,
            settings::reset_settings,
            settings::config_path,
            reveal_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
