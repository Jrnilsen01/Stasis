mod footprint;
mod logging;
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
        .setup(|app| {
            if let Err(e) = logging::init(app.handle()) {
                // A logger that could not start is not a reason to refuse to
                // open the window. Everything the app does still works; the
                // next bug report is just thinner.
                eprintln!("logging is unavailable: {e}");
            }
            log::info!("stasis {} started", app.package_info().version);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            footprint::read_footprint,
            steam::scan_steam_games,
            modules::list_modules,
            modules::modules_dir,
            settings::load_settings,
            settings::save_settings,
            settings::reset_settings,
            settings::check_steam_path,
            settings::config_path,
            logging::log_path,
            reveal_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
