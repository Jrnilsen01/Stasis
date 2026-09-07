use std::path::{Path, PathBuf};

use serde::Serialize;

/// One installed game, read from Steam's own manifests. Every field here comes
/// off disk. Nothing is estimated, so a game with no recorded play time reports
/// `None` rather than a plausible-looking date.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub app_id: String,
    pub name: String,
    /// Bytes, as Steam recorded them. `None` when the manifest omits the field.
    pub size_on_disk: Option<u64>,
    /// Unix seconds. `None` when never played or not recorded.
    pub last_played: Option<i64>,
    /// Which library folder it lives in, so multi-drive setups stay legible.
    pub library: String,
}

/// The scan distinguishes three outcomes because they are three different
/// screens: no Steam at all, Steam with an empty library, and a real list.
#[derive(Serialize)]
#[serde(tag = "status", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ScanResult {
    /// Steam itself was not found, so there is nothing to read.
    SteamNotFound {
        searched: Vec<String>,
        /// True when the user set an explicit folder and that folder was wrong,
        /// which needs different wording from "auto-detection came up empty".
        overridden: bool,
    },
    /// Steam was found and read successfully. `games` may legitimately be empty.
    Scanned {
        steam_root: String,
        libraries: Vec<String>,
        games: Vec<Game>,
    },
    /// Steam was found but could not be read (permissions, corrupt manifest).
    Unreadable { path: String, reason: String },
}

/// Pulls the value that follows a quoted key on the same line of a VDF file.
/// Steam's key-values format is regular enough that a full parser would be more
/// machinery than the two fields we need.
fn vdf_values(content: &str, key: &str) -> Vec<String> {
    let needle = format!("\"{key}\"");
    let mut found = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.to_ascii_lowercase().starts_with(&needle.to_ascii_lowercase()) {
            continue;
        }
        let after_key = &trimmed[needle.len()..];
        let mut parts = after_key.split('"');
        parts.next();
        if let Some(value) = parts.next() {
            found.push(value.replace("\\\\", "\\"));
        }
    }

    found
}

fn vdf_value(content: &str, key: &str) -> Option<String> {
    vdf_values(content, key).into_iter().next()
}

/// Candidate Steam roots, most authoritative first. The registry knows where a
/// relocated install went; the fixed paths only cover the default case.
fn candidate_roots(override_path: Option<&str>) -> Vec<PathBuf> {
    // An explicit folder is authoritative. Falling back to auto-detection when
    // it is wrong would quietly scan a different folder than the one the user
    // named, so the setting would look like it did nothing.
    if let Some(path) = override_path {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return vec![PathBuf::from(trimmed)];
        }
    }

    let mut candidates = Vec::new();

    #[cfg(target_os = "windows")]
    {
        use winreg::enums::HKEY_CURRENT_USER;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(key) = hkcu.open_subkey(r"Software\Valve\Steam") {
            if let Ok(path) = key.get_value::<String, _>("SteamPath") {
                candidates.push(PathBuf::from(path));
            }
        }
        candidates.push(PathBuf::from(r"C:\Program Files (x86)\Steam"));
        candidates.push(PathBuf::from(r"C:\Program Files\Steam"));
    }

    #[cfg(target_os = "macos")]
    if let Some(home) = std::env::var_os("HOME") {
        candidates.push(PathBuf::from(home).join("Library/Application Support/Steam"));
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        candidates.push(home.join(".steam/steam"));
        candidates.push(home.join(".local/share/Steam"));
    }

    candidates
}

/// Every library folder Steam knows about, including the root itself. Games on a
/// second drive are the normal case, not an edge case.
fn library_folders(steam_root: &Path) -> Vec<PathBuf> {
    let mut candidates = vec![steam_root.to_path_buf()];

    let manifest = steam_root.join("steamapps").join("libraryfolders.vdf");
    if let Ok(content) = std::fs::read_to_string(&manifest) {
        for path in vdf_values(&content, "path") {
            candidates.push(PathBuf::from(path));
        }
    }

    // Steam lists its own root inside libraryfolders.vdf, and the registry
    // spells that same folder differently: "c:/program files (x86)/steam"
    // against "C:\Program Files (x86)\Steam". Comparing the raw paths treats
    // them as two libraries and lists every installed game twice, so the
    // deduplication has to happen on the canonical form.
    let mut libraries: Vec<PathBuf> = Vec::new();
    let mut seen: Vec<PathBuf> = Vec::new();

    for candidate in candidates {
        let key = std::fs::canonicalize(&candidate).unwrap_or_else(|_| candidate.clone());
        if !seen.contains(&key) {
            seen.push(key);
            libraries.push(candidate);
        }
    }

    libraries
}

fn games_in_library(library: &Path) -> Vec<Game> {
    let steamapps = library.join("steamapps");
    let Ok(entries) = std::fs::read_dir(&steamapps) else {
        return Vec::new();
    };

    let mut games = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let is_manifest = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with("appmanifest_") && n.ends_with(".acf"));
        if !is_manifest {
            continue;
        }

        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };

        // A manifest without a name is a partial download, not an installed game.
        let (Some(app_id), Some(name)) = (vdf_value(&content, "appid"), vdf_value(&content, "name"))
        else {
            continue;
        };

        games.push(Game {
            app_id,
            name,
            size_on_disk: vdf_value(&content, "SizeOnDisk").and_then(|v| v.parse().ok()),
            last_played: vdf_value(&content, "LastPlayed")
                .and_then(|v| v.parse().ok())
                .filter(|&ts: &i64| ts > 0),
            library: library.display().to_string(),
        });
    }

    games
}

#[tauri::command]
pub fn scan_steam_games(override_path: Option<String>) -> ScanResult {
    let overridden = override_path
        .as_deref()
        .is_some_and(|p| !p.trim().is_empty());
    let candidates = candidate_roots(override_path.as_deref());

    let Some(steam_root) = candidates.iter().find(|p| p.join("steamapps").is_dir()) else {
        return ScanResult::SteamNotFound {
            searched: candidates.iter().map(|p| p.display().to_string()).collect(),
            overridden,
        };
    };

    let libraries = library_folders(steam_root);

    // A root that exists but cannot be listed is a permissions problem, and the
    // user needs to be told that rather than shown an empty library.
    if let Err(e) = std::fs::read_dir(steam_root.join("steamapps")) {
        return ScanResult::Unreadable {
            path: steam_root.display().to_string(),
            reason: e.to_string(),
        };
    }

    let mut games: Vec<Game> = libraries.iter().flat_map(|lib| games_in_library(lib)).collect();
    games.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    ScanResult::Scanned {
        steam_root: steam_root.display().to_string(),
        libraries: libraries.iter().map(|p| p.display().to_string()).collect(),
        games,
    }
}
