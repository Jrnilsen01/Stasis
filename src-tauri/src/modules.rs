use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::logging::redact;

/// An installed overlay module, read from its own manifest on disk.
///
/// There is no bundled module registry, so on a fresh install this list is
/// genuinely empty. The UI shows that emptiness honestly instead of shipping
/// sample modules that would look installed without being installed.
#[derive(Serialize, Deserialize, Debug)]
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
    std::fs::create_dir_all(&dir).map_err(|e| {
        log::error!("{} could not be created: {e}", redact(&dir));
        format!("could not create {}: {e}", dir.display())
    })?;
    Ok(dir)
}

#[tauri::command]
pub fn modules_dir(app: tauri::AppHandle) -> Result<String, String> {
    modules_root(&app).map(|p| p.display().to_string())
}

#[tauri::command]
pub fn list_modules(app: tauri::AppHandle) -> Result<Vec<Module>, String> {
    read_modules(&modules_root(&app)?)
}

/// Reads every module manifest directly under `root`.
///
/// Split out from `list_modules` because the `AppHandle` there only supplies a
/// path. Keeping the reading separate makes it testable against a real folder
/// rather than a mocked application.
fn read_modules(root: &Path) -> Result<Vec<Module>, String> {
    let entries = std::fs::read_dir(root).map_err(|e| {
        log::error!("{} could not be listed: {e}", redact(root));
        format!("could not read {}: {e}", root.display())
    })?;

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
                log::error!("{} did not parse: {e}", redact(&manifest));
                return Err(format!(
                    "{} is not a valid module manifest: {e}",
                    manifest.display()
                ));
            }
        }
    }

    modules.sort_by_key(|m| m.name.to_lowercase());
    Ok(modules)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    const VALID: &str = r#"{
  "id": "crosshair",
  "name": "Crosshair",
  "summary": "A static crosshair overlay",
  "version": "1.0.0"
}"#;

    /// Builds a modules root from `(folder name, file name, contents)` triples.
    fn modules_root_with(entries: &[(&str, &str, &str)]) -> TempDir {
        let dir = TempDir::new().expect("temp dir");
        for (folder, file, contents) in entries {
            let path = dir.path().join(folder);
            fs::create_dir_all(&path).expect("module folder");
            fs::write(path.join(file), contents).expect("write manifest");
        }
        dir
    }

    #[test]
    fn reads_a_valid_manifest() {
        let dir = modules_root_with(&[("crosshair", "module.json", VALID)]);
        let modules = read_modules(dir.path()).expect("should read");

        assert_eq!(modules.len(), 1);
        assert_eq!(modules[0].id, "crosshair");
        assert_eq!(modules[0].name, "Crosshair");
        assert_eq!(modules[0].summary, "A static crosshair overlay");
        assert_eq!(modules[0].version, "1.0.0");
    }

    #[test]
    fn path_is_filled_in_from_disk_not_from_the_manifest() {
        // `path` is skip_deserializing, so a manifest cannot claim to live
        // somewhere it does not. The value has to come from the folder itself.
        let manifest = r#"{
  "id": "liar",
  "name": "Liar",
  "summary": "Claims a path it does not have",
  "version": "1.0.0",
  "path": "C:\\Windows\\System32"
}"#;
        let dir = modules_root_with(&[("liar", "module.json", manifest)]);
        let modules = read_modules(dir.path()).expect("should read");

        assert_eq!(modules.len(), 1);
        assert_ne!(modules[0].path, r"C:\Windows\System32");
        assert_eq!(
            modules[0].path,
            dir.path().join("liar").display().to_string()
        );
    }

    #[test]
    fn a_malformed_manifest_is_an_error_rather_than_a_silent_skip() {
        // Intended behaviour: a folder that is trying to be a module and failing
        // should say so. Skipping it would leave the user staring at an empty
        // list with no reason given.
        let dir = modules_root_with(&[("broken", "module.json", "{ not json")]);
        let error = read_modules(dir.path()).expect_err("should fail");

        assert!(
            error.contains("is not a valid module manifest"),
            "unhelpful error: {error}"
        );
        assert!(
            error.contains("broken"),
            "error should name the folder: {error}"
        );
    }

    #[test]
    fn a_manifest_missing_a_required_field_is_also_an_error() {
        let manifest = r#"{ "id": "partial", "name": "Partial" }"#;
        let dir = modules_root_with(&[("partial", "module.json", manifest)]);

        assert!(read_modules(dir.path()).is_err());
    }

    #[test]
    fn a_folder_without_a_manifest_is_skipped_quietly() {
        // The modules folder is somewhere the user can drop things. Notes and
        // archives in there are not broken modules.
        let dir = modules_root_with(&[
            ("crosshair", "module.json", VALID),
            ("my-notes", "readme.txt", "not a module"),
        ]);

        assert_eq!(read_modules(dir.path()).expect("should read").len(), 1);
    }

    #[test]
    fn sorting_is_case_insensitive_by_name() {
        let named = |id: &str, name: &str| {
            format!(r#"{{ "id": "{id}", "name": "{name}", "summary": "s", "version": "1.0.0" }}"#)
        };
        let zebra = named("z", "zebra");
        let apple = named("a", "Apple");
        let middle = named("m", "middle");
        let dir = modules_root_with(&[
            ("z", "module.json", &zebra),
            ("a", "module.json", &apple),
            ("m", "module.json", &middle),
        ]);

        let names: Vec<String> = read_modules(dir.path())
            .expect("should read")
            .into_iter()
            .map(|m| m.name)
            .collect();

        assert_eq!(names, vec!["Apple", "middle", "zebra"]);
    }

    #[test]
    fn an_empty_folder_is_an_empty_list_not_an_error() {
        // The fresh-install case. It has to succeed so the UI can show its
        // empty state rather than an error.
        let dir = TempDir::new().expect("temp dir");
        assert!(read_modules(dir.path()).expect("should read").is_empty());
    }

    #[test]
    fn a_missing_folder_is_an_error_that_names_the_path() {
        let dir = TempDir::new().expect("temp dir");
        let missing = dir.path().join("does-not-exist");
        let error = read_modules(&missing).expect_err("should fail");

        assert!(error.contains("could not read"), "unhelpful error: {error}");
        assert!(
            error.contains("does-not-exist"),
            "error should name the path: {error}"
        );
    }
}
