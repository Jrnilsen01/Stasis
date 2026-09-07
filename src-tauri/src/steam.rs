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
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
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
        if !trimmed
            .to_ascii_lowercase()
            .starts_with(&needle.to_ascii_lowercase())
        {
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
        let (Some(app_id), Some(name)) =
            (vdf_value(&content, "appid"), vdf_value(&content, "name"))
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

    let mut games: Vec<Game> = libraries
        .iter()
        .flat_map(|lib| games_in_library(lib))
        .collect();
    games.sort_by_key(|g| g.name.to_lowercase());

    ScanResult::Scanned {
        steam_root: steam_root.display().to_string(),
        libraries: libraries.iter().map(|p| p.display().to_string()).collect(),
        games,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    const MANIFEST: &str = r#"
"AppState"
{
	"appid"		"220"
	"Universe"		"1"
	"name"		"Half-Life 2"
	"StateFlags"		"4"
	"installdir"		"Half-Life 2"
	"LastUpdated"		"1698240000"
	"SizeOnDisk"		"7538044108"
	"LastPlayed"		"1700000000"
}
"#;

    // --- vdf_values -------------------------------------------------------

    #[test]
    fn reads_the_fields_a_manifest_actually_carries() {
        assert_eq!(vdf_value(MANIFEST, "appid").as_deref(), Some("220"));
        assert_eq!(vdf_value(MANIFEST, "name").as_deref(), Some("Half-Life 2"));
        assert_eq!(
            vdf_value(MANIFEST, "SizeOnDisk").as_deref(),
            Some("7538044108")
        );
        assert_eq!(
            vdf_value(MANIFEST, "LastPlayed").as_deref(),
            Some("1700000000")
        );
    }

    #[test]
    fn key_matching_ignores_case() {
        // Steam has not been consistent about casing across manifest versions,
        // so "sizeondisk" and "SizeOnDisk" have to reach the same value.
        assert_eq!(
            vdf_value(MANIFEST, "sizeondisk"),
            vdf_value(MANIFEST, "SizeOnDisk")
        );
        assert_eq!(vdf_value(MANIFEST, "APPID").as_deref(), Some("220"));
    }

    #[test]
    fn a_key_that_prefixes_another_key_does_not_match_it() {
        // "name" must not pick up "name_localized". The closing quote inside the
        // needle is what prevents it, so this locks that in.
        let content = r#"
	"name_localized"		"Halbwertszeit 2"
	"name"		"Half-Life 2"
"#;
        assert_eq!(vdf_value(content, "name").as_deref(), Some("Half-Life 2"));
    }

    #[test]
    fn unescapes_the_doubled_backslashes_steam_writes() {
        let content = r#""path"		"D:\\SteamLibrary\\Games""#;
        assert_eq!(
            vdf_value(content, "path").as_deref(),
            Some(r"D:\SteamLibrary\Games")
        );
    }

    #[test]
    fn returns_every_occurrence_in_file_order() {
        let content = r#"
"libraryfolders"
{
	"0"
	{
		"path"		"C:\\Steam"
	}
	"1"
	{
		"path"		"D:\\SteamLibrary"
	}
}
"#;
        assert_eq!(
            vdf_values(content, "path"),
            vec![r"C:\Steam", r"D:\SteamLibrary"]
        );
    }

    #[test]
    fn a_missing_key_yields_nothing_rather_than_an_empty_string() {
        assert!(vdf_values(MANIFEST, "BetaKey").is_empty());
        assert_eq!(vdf_value(MANIFEST, "BetaKey"), None);
    }

    // --- candidate_roots --------------------------------------------------

    #[test]
    fn an_override_is_the_only_candidate() {
        // The whole point of the setting: auto-detection must not run behind it
        // and quietly scan a different folder than the one the user named.
        let roots = candidate_roots(Some(r"D:\Games\Steam"));
        assert_eq!(roots, vec![PathBuf::from(r"D:\Games\Steam")]);
    }

    #[test]
    fn an_override_is_trimmed() {
        let roots = candidate_roots(Some("  D:\\Games\\Steam  "));
        assert_eq!(roots, vec![PathBuf::from(r"D:\Games\Steam")]);
    }

    #[test]
    fn a_blank_override_falls_back_to_auto_detection() {
        // An empty settings field means "unset", not "scan the empty path".
        for blank in ["", "   ", "\t"] {
            let roots = candidate_roots(Some(blank));
            assert!(
                !roots.contains(&PathBuf::from(blank)),
                "blank override {blank:?} was treated as a real path"
            );
        }
    }

    // --- library_folders --------------------------------------------------

    fn steam_root_with(manifest: Option<&str>) -> TempDir {
        let dir = TempDir::new().expect("temp dir");
        let steamapps = dir.path().join("steamapps");
        fs::create_dir_all(&steamapps).expect("steamapps");
        if let Some(content) = manifest {
            fs::write(steamapps.join("libraryfolders.vdf"), content).expect("write manifest");
        }
        dir
    }

    #[test]
    fn the_root_counts_as_a_library_even_with_no_manifest() {
        let dir = steam_root_with(None);
        assert_eq!(library_folders(dir.path()), vec![dir.path().to_path_buf()]);
    }

    #[test]
    fn a_second_drive_is_listed_alongside_the_root() {
        let other = TempDir::new().expect("temp dir");
        let listed = other.path().display().to_string().replace('\\', r"\\");
        let dir = steam_root_with(Some(&format!("\"path\"\t\t\"{listed}\"")));

        assert_eq!(library_folders(dir.path()).len(), 2);
    }

    #[test]
    #[cfg(windows)]
    fn the_root_listed_back_in_its_own_manifest_is_not_a_second_library() {
        // The bug this guards: Steam records its own root inside
        // libraryfolders.vdf spelled differently from the registry, so comparing
        // the raw paths counted one folder twice and listed every game twice.
        let dir = steam_root_with(None);
        let root = dir.path().display().to_string();
        let restyled = root.to_lowercase().replace('\\', "/");
        assert_ne!(root, restyled, "the test needs the two spellings to differ");

        fs::write(
            dir.path().join("steamapps").join("libraryfolders.vdf"),
            format!("\"path\"\t\t\"{restyled}\""),
        )
        .expect("write manifest");

        assert_eq!(
            library_folders(dir.path()),
            vec![dir.path().to_path_buf()],
            "the same folder in two spellings was counted twice"
        );
    }

    #[test]
    fn a_library_that_no_longer_exists_is_still_reported() {
        // Canonicalisation cannot resolve a drive that is gone, so the raw path
        // is kept. Dropping it silently would hide a library the user still has
        // configured.
        let dir = steam_root_with(Some(r#""path"		"Z:\\GoneLibrary""#));
        let libraries = library_folders(dir.path());

        assert_eq!(libraries.len(), 2);
        assert!(libraries.contains(&PathBuf::from(r"Z:\GoneLibrary")));
    }

    // --- games_in_library -------------------------------------------------

    fn library_with(manifests: &[(&str, &str)]) -> TempDir {
        let dir = TempDir::new().expect("temp dir");
        let steamapps = dir.path().join("steamapps");
        fs::create_dir_all(&steamapps).expect("steamapps");
        for (filename, content) in manifests {
            fs::write(steamapps.join(filename), content).expect("write manifest");
        }
        dir
    }

    #[test]
    fn reads_a_game_out_of_a_real_manifest() {
        let dir = library_with(&[("appmanifest_220.acf", MANIFEST)]);
        let games = games_in_library(dir.path());

        assert_eq!(games.len(), 1);
        assert_eq!(games[0].app_id, "220");
        assert_eq!(games[0].name, "Half-Life 2");
        assert_eq!(games[0].size_on_disk, Some(7_538_044_108));
        assert_eq!(games[0].last_played, Some(1_700_000_000));
    }

    #[test]
    fn a_never_played_game_reports_none_rather_than_the_epoch() {
        // Steam writes 0 for "never played". Passing that through as a date
        // would render as 1 January 1970 in the UI.
        let manifest = r#"
	"appid"		"400"
	"name"		"Portal"
	"LastPlayed"		"0"
"#;
        let dir = library_with(&[("appmanifest_400.acf", manifest)]);
        let games = games_in_library(dir.path());

        assert_eq!(games.len(), 1);
        assert_eq!(games[0].last_played, None);
    }

    #[test]
    fn a_manifest_with_no_name_is_a_partial_download_and_is_skipped() {
        let manifest = r#"
	"appid"		"620"
	"StateFlags"		"1026"
"#;
        let dir = library_with(&[("appmanifest_620.acf", manifest)]);
        assert!(games_in_library(dir.path()).is_empty());
    }

    #[test]
    fn a_missing_size_is_none_rather_than_zero() {
        let manifest = r#"
	"appid"		"70"
	"name"		"Half-Life"
"#;
        let dir = library_with(&[("appmanifest_70.acf", manifest)]);
        let games = games_in_library(dir.path());

        assert_eq!(games[0].size_on_disk, None);
    }

    #[test]
    fn files_that_are_not_app_manifests_are_ignored() {
        let dir = library_with(&[
            ("appmanifest_220.acf", MANIFEST),
            ("libraryfolders.vdf", r#""path"		"C:\\Steam""#),
            ("appmanifest_220.acf.bak", MANIFEST),
            ("workshop_220.acf", MANIFEST),
        ]);

        assert_eq!(games_in_library(dir.path()).len(), 1);
    }

    #[test]
    fn a_library_with_no_steamapps_folder_yields_no_games() {
        let dir = TempDir::new().expect("temp dir");
        assert!(games_in_library(dir.path()).is_empty());
    }
}
