use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::Manager;

/// An installed overlay module, read from its own manifest on disk.
///
/// There is no bundled module registry, so on a fresh install this list is
/// genuinely empty. The UI shows that emptiness honestly instead of shipping
/// sample modules that would look installed without being installed.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Module {
    pub id: String,
    pub name: String,
    pub summary: String,
    pub version: String,
    #[serde(skip_deserializing)]
    pub path: String,
}

fn modules_root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("no data directory available: {e}"))?
        .join("modules");
    std::fs::create_dir_all(&dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    Ok(dir)
}

#[tauri::command]
pub fn modules_dir(app: tauri::AppHandle) -> Result<String, String> {
    modules_root(&app).map(|p| p.display().to_string())
}

#[tauri::command]
pub fn list_modules(app: tauri::AppHandle) -> Result<Vec<Module>, String> {
    let root = modules_root(&app)?;
    let entries = std::fs::read_dir(&root)
        .map_err(|e| format!("could not read {}: {e}", root.display()))?;

    let mut modules = Vec::new();

    for entry in entries.flatten() {
        let manifest = entry.path().join("module.json");
        let Ok(raw) = std::fs::read_to_string(&manifest) else {
            // A folder without a manifest is not a module. Skipping it quietly
            // is correct: the user may keep notes or archives in here.
            continue;
        };
        match serde_json::from_str::<Module>(&raw) {
            Ok(mut module) => {
                module.path = entry.path().display().to_string();
                modules.push(module);
            }
            Err(e) => {
                return Err(format!("{} is not a valid module manifest: {e}", manifest.display()));
            }
        }
    }

    modules.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(modules)
}
