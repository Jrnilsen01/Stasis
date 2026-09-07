//! The kill switch: a way to stop injection without the Stasis UI.
//!
//! The reasoning, from the project's own TODO: if a hook misbehaves mid
//! session, the route out cannot run through the thing that is misbehaving. A
//! user whose game has just started stuttering because of our overlay should
//! not have to alt-tab into a Tauri window that may itself be wedged. They need
//! a route that works from outside the app entirely.
//!
//! There are two markers, checked in this order, because they suit two
//! different moments:
//!
//! 1. The `STASIS_DISABLE` environment variable, set to any non-empty value.
//!    This is the convenient one: set it before launching, and no injection
//!    happens for the life of that shell. It cannot be changed for a process
//!    that is already running, which is exactly why the file exists too.
//!
//! 2. A marker file on disk. This is the mid-session route. Create an empty
//!    file at the path below with anything at hand, Explorer, Notepad, a
//!    terminal, and the next injection attempt refuses. Its default location
//!    is `%TEMP%\stasis-disable`, and the path can be overridden with
//!    `STASIS_DISABLE_FILE` for anyone who wants it somewhere they can reach
//!    faster.
//!
//! Both sides check this. The injector calls [`kill_switch_engaged`] before it
//! touches a target, so an engaged switch means nothing is ever injected. The
//! payload checks the same markers inside its own `DllMain` and refuses to load
//! if either is present, so even a DLL that somehow reached the process still
//! declines to run. Two independent checks, because the failure this guards
//! against is precisely the case where one layer is not behaving.

use std::path::{Path, PathBuf};

use crate::error::KillSwitchReason;

/// The environment variable that disables injection when set to a non-empty
/// value.
pub const DISABLE_ENV: &str = "STASIS_DISABLE";

/// The environment variable that overrides where the marker file is looked for.
pub const DISABLE_FILE_ENV: &str = "STASIS_DISABLE_FILE";

/// The marker file's name under the temp directory when no override is given.
pub const DEFAULT_MARKER_NAME: &str = "stasis-disable";

/// The path the marker file is looked for at: the `STASIS_DISABLE_FILE`
/// override if set, otherwise `stasis-disable` in the system temp directory.
pub fn marker_path() -> PathBuf {
    match std::env::var_os(DISABLE_FILE_ENV) {
        Some(path) if !path.is_empty() => PathBuf::from(path),
        _ => std::env::temp_dir().join(DEFAULT_MARKER_NAME),
    }
}

/// Report whether injection is disabled, and by which marker, or `None` if it
/// is allowed. The injector treats `Some` as a hard refusal.
pub fn kill_switch_engaged() -> Option<KillSwitchReason> {
    let env_value = std::env::var(DISABLE_ENV).ok();
    evaluate(env_value.as_deref(), &marker_path())
}

/// The decision itself, split out from the sources so it can be tested with
/// controlled inputs rather than by mutating this process's real environment.
/// The environment variable is checked first so that setting it always wins,
/// regardless of what happens to be on disk.
fn evaluate(disable_env: Option<&str>, marker: &Path) -> Option<KillSwitchReason> {
    if let Some(value) = disable_env {
        if !value.is_empty() {
            return Some(KillSwitchReason::EnvironmentVariable);
        }
    }
    if marker.exists() {
        return Some(KillSwitchReason::MarkerFile(marker.to_path_buf()));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_marker(name: &str) -> PathBuf {
        // A per-test file name under the temp dir keeps these from colliding
        // when the suite runs in parallel, and avoids touching the real default
        // marker that a developer might have set to disable their own machine.
        std::env::temp_dir().join(format!("stasis-killswitch-test-{name}"))
    }

    #[test]
    fn injection_is_allowed_when_no_marker_is_present() {
        let marker = temp_marker("absent");
        let _ = fs::remove_file(&marker);
        assert_eq!(evaluate(None, &marker), None);
    }

    #[test]
    fn a_non_empty_environment_variable_engages_the_switch() {
        let marker = temp_marker("env-set");
        let _ = fs::remove_file(&marker);
        assert_eq!(
            evaluate(Some("1"), &marker),
            Some(KillSwitchReason::EnvironmentVariable)
        );
    }

    #[test]
    fn an_empty_environment_variable_does_not_engage_the_switch() {
        // An empty value is treated as unset rather than as disabled, so that
        // clearing the variable rather than removing it has the effect the user
        // expects, which is to turn injection back on.
        let marker = temp_marker("env-empty");
        let _ = fs::remove_file(&marker);
        assert_eq!(evaluate(Some(""), &marker), None);
    }

    #[test]
    fn the_marker_file_engages_the_switch_and_the_reason_names_it() {
        let marker = temp_marker("file-present");
        fs::write(&marker, b"").expect("write marker");
        let reason = evaluate(None, &marker);
        assert_eq!(reason, Some(KillSwitchReason::MarkerFile(marker.clone())));
        fs::remove_file(&marker).expect("clean up marker");
    }

    #[test]
    fn the_environment_variable_is_reported_ahead_of_the_marker_file() {
        // When both are present the message should name the one the user is most
        // likely to have set on purpose in that shell, which is the variable.
        let marker = temp_marker("both");
        fs::write(&marker, b"").expect("write marker");
        assert_eq!(
            evaluate(Some("1"), &marker),
            Some(KillSwitchReason::EnvironmentVariable)
        );
        fs::remove_file(&marker).expect("clean up marker");
    }
}
