use std::path::{Path, PathBuf};

use tauri::Manager;
use tauri_plugin_log::{RotationStrategy, Target, TargetKind};

/// Name of the log file, without the extension the plugin appends.
const FILE_STEM: &str = "stasis";

/// One megabyte, then the file starts over.
///
/// This is a tool people leave open for a working day, so a log with no ceiling
/// is a disk that fills while nobody is watching. A megabyte is far more than a
/// normal session writes and still small enough to attach to an issue without
/// thinking about it.
const MAX_LOG_BYTES: u128 = 1024 * 1024;

fn logs_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|dir| dir.join("logs"))
        .map_err(|e| format!("no data directory available: {e}"))
}

/// Starts the file logger.
///
/// Registered from `setup` rather than on the builder because the folder is
/// resolved from this app's own data directory, which needs an `AppHandle`.
/// Tauri's own log directory would put the file under LocalAppData while
/// `settings.json` and the modules folder sit under Roaming, and one folder a
/// user can be pointed at beats the platform-default split across two.
pub fn init(app: &tauri::AppHandle) -> Result<(), String> {
    let plugin = tauri_plugin_log::Builder::new()
        // The defaults include Tauri's log directory. Clearing them first is
        // what keeps this to the one file the Settings screen names.
        .clear_targets()
        .target(Target::new(TargetKind::Folder {
            path: logs_dir(app)?,
            file_name: Some(FILE_STEM.to_string()),
        }))
        // Shows up in the terminal under `tauri dev`. A release build has no
        // console attached, so it costs nothing there.
        .target(Target::new(TargetKind::Stdout))
        .max_file_size(MAX_LOG_BYTES)
        .rotation_strategy(RotationStrategy::KeepOne)
        .level(log::LevelFilter::Info)
        .build();

    app.plugin(plugin).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn log_path(app: tauri::AppHandle) -> Result<String, String> {
    Ok(logs_dir(&app)?
        .join(format!("{FILE_STEM}.log"))
        .display()
        .to_string())
}

/// Strips the user's home folder off a path on its way into the log.
///
/// The whole point of the file is that someone can attach it to a public issue,
/// and `C:\Users\their-real-name` on every line is a detail they did not choose
/// to publish. What is left still names the folder, which is the part that
/// helps whoever reads the report.
pub fn redact(path: &Path) -> String {
    let text = path.display().to_string();
    match home_dir() {
        Some(home) => strip_home(&text, &home),
        None => text,
    }
}

fn home_dir() -> Option<String> {
    let key = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    std::env::var(key).ok().filter(|value| !value.is_empty())
}

fn strip_home(path: &str, home: &str) -> String {
    let home = home.trim_end_matches(['\\', '/']);
    if home.is_empty() {
        return path.to_string();
    }

    // Windows hands the same folder back in different cases and with either
    // separator depending on who was asked, so both are normalised before the
    // prefix is compared.
    let fold = |c: char| {
        if c == '\\' {
            '/'
        } else {
            c.to_ascii_lowercase()
        }
    };

    let mut rest = path.chars();
    for expected in home.chars() {
        match rest.next() {
            Some(actual) if fold(actual) == fold(expected) => {}
            _ => return path.to_string(),
        }
    }

    match rest.clone().next() {
        // A folder that merely starts with the same letters is a different
        // folder: `C:\Usersomething` is not inside `C:\Users`.
        Some(c) if c != '\\' && c != '/' => path.to_string(),
        Some(_) => format!("~{}", rest.as_str()),
        None => "~".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_inside_the_home_folder_loses_the_user_name() {
        assert_eq!(
            strip_home(
                r"C:\Users\alice\AppData\Roaming\com.joaki.stasis",
                r"C:\Users\alice"
            ),
            r"~\AppData\Roaming\com.joaki.stasis"
        );
    }

    #[test]
    fn a_path_outside_the_home_folder_is_left_alone() {
        // The part of a Steam path that matters for a bug report is which drive
        // and folder it is, and none of that identifies anyone.
        assert_eq!(
            strip_home(r"D:\SteamLibrary", r"C:\Users\alice"),
            r"D:\SteamLibrary"
        );
    }

    #[test]
    fn the_prefix_match_ignores_case_and_separator_style() {
        // The registry, the environment, and std all spell the same folder
        // differently, so a literal comparison would miss most of the time.
        assert_eq!(
            strip_home(r"c:\users\alice\Documents", r"C:/Users/Alice"),
            r"~\Documents"
        );
    }

    #[test]
    fn a_sibling_that_merely_starts_with_the_home_path_is_not_stripped() {
        assert_eq!(
            strip_home(r"C:\Users\alice-backup\Steam", r"C:\Users\alice"),
            r"C:\Users\alice-backup\Steam"
        );
    }

    #[test]
    fn the_home_folder_itself_becomes_a_single_tilde() {
        assert_eq!(strip_home(r"C:\Users\alice", r"C:\Users\alice\"), "~");
    }

    #[test]
    fn an_empty_home_leaves_every_path_untouched() {
        // Reading the environment can fail. Falling back to a bare `~` prefix
        // would rewrite every path in the log into nonsense.
        assert_eq!(strip_home(r"D:\Games", ""), r"D:\Games");
    }
}
