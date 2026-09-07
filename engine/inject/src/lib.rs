//! Loading the Stasis overlay payload into a target process, and reporting
//! honestly when it cannot.
//!
//! This is the part of Stasis that most resembles a cheat to an outside
//! observer, and the project's own TODO says so. It is written to be read: only
//! documented Win32 calls, one failure at a time, and comments that explain why
//! rather than restating what. Someone auditing this crate to decide whether
//! Stasis is trustworthy should be able to follow every step and come away
//! reassured. Nothing here reads a game's memory for gameplay state; the only
//! memory written to the target is a buffer holding the payload's own file path.
//!
//! # Never point this at a real game during development
//!
//! Injecting an unsigned, unknown DLL into an anti-cheat protected process is
//! how accounts get banned. The only supported development target is the D3D11
//! harness in `engine/harness`, which presents frames like a game without being
//! one. The tests in this crate target the harness and nothing else.
//!
//! # The process model
//!
//! Stasis hooks the graphics API, and that decision partly makes this one for
//! us. A `Present` hook has to run on the game's own render thread, in the
//! game's address space, because that is where the swap chain is. So the
//! drawing code lives inside the game whether we like it or not. What is still
//! a choice is how much else moves in with it, and the answer taken here is: as
//! little as can be defended.
//!
//! The split is therefore:
//!
//! - **Inside the game (the payload, a separate `cdylib`):** the `Present`
//!   hook, the drawing, a signal that it is up, and a check of the kill switch
//!   so it can refuse to load. Nothing more. Keeping the in game surface this
//!   small is the point: every capability added there is capability that has to
//!   be trusted there, inside someone else's protected process.
//! - **In the Stasis process (this crate and its callers, out of process):**
//!   everything else. Finding the target, deciding whether to inject at all,
//!   performing the injection, and owning the lifecycle. The controller does
//!   not have to live in the game, so it does not, which is what gives the
//!   misbehaving-hook case somewhere safe to recover from.
//! - **How they talk:** at injection time, a named event carries the single bit
//!   that matters, that the payload is up (see [`readiness`]). The kill switch
//!   is a file and an environment marker that both sides read independently
//!   (see [`killswitch`]). A richer control channel is deliberately not built
//!   into the payload yet.
//!
//! # Turning it off without the app
//!
//! If a hook misbehaves mid session, the route out cannot run through the app
//! that is misbehaving. Set the `STASIS_DISABLE` environment variable, or create
//! an empty file named `stasis-disable` in your temp directory, and no further
//! injection happens; the payload checks the same markers and refuses to load
//! too. The full explanation, including how to move the marker file, is in the
//! [`killswitch`] module, which is written to be found while things are broken.

use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

mod error;
mod killswitch;
mod process;
mod readiness;
mod remote;

pub use error::{Bitness, InjectError, KillSwitchReason};
pub use killswitch::{
    kill_switch_engaged, marker_path, DEFAULT_MARKER_NAME, DISABLE_ENV, DISABLE_FILE_ENV,
};
pub use process::injector_bitness;
pub use readiness::event_name as readiness_event_name;

/// Which process to inject into.
///
/// By pid when the caller already knows the exact instance (the harness prints
/// its own), or by executable name when it does not. Name lookup returns the
/// first match, so a caller that must have one specific instance among several
/// of the same name should resolve the pid itself and pass [`Target::Pid`].
pub enum Target {
    Pid(u32),
    Name(String),
}

/// Knobs for an injection, with defaults that suit the harness and a typical
/// game.
pub struct InjectOptions {
    /// How long to wait, after the DLL has loaded, for the payload to report
    /// readiness before giving up with [`InjectError::PayloadNotReady`]. The
    /// same budget bounds the load itself, so a payload that hangs in its own
    /// initialisation is caught rather than waited on forever.
    pub readiness_timeout: Duration,
}

impl Default for InjectOptions {
    fn default() -> Self {
        // Five seconds is long enough for a real payload to finish its own
        // start-up on a busy machine, and short enough that a failure is
        // reported while the user is still looking at the screen.
        InjectOptions {
            readiness_timeout: Duration::from_secs(5),
        }
    }
}

/// The result of a successful injection.
#[derive(Debug)]
pub struct Injected {
    /// The process the payload was loaded into.
    pub pid: u32,
    /// Whether the payload signalled readiness. Always true on success today,
    /// but carried explicitly so a future caller that opts out of the readiness
    /// wait can tell a confirmed load from an unconfirmed one rather than
    /// guessing.
    pub readiness_confirmed: bool,
}

/// Load the payload DLL into the target, checking every precondition first and
/// returning a specific [`InjectError`] for whichever one fails.
///
/// The order is deliberate. The cheapest and most decisive refusals come first:
/// the kill switch (a chosen "do nothing" state), then the DLL actually
/// existing, then the target. Only once all of those hold does anything touch
/// the target's memory.
pub fn inject(
    target: &Target,
    dll: &Path,
    options: &InjectOptions,
) -> Result<Injected, InjectError> {
    // The kill switch is checked before anything else, so an engaged switch
    // means Stasis provably does nothing to any process.
    if let Some(reason) = kill_switch_engaged() {
        return Err(InjectError::KillSwitchEngaged { reason });
    }

    // Resolve the DLL to an absolute path and confirm it is on disk. A relative
    // or missing path would otherwise fail confusingly deep inside the target,
    // where the error would only say the load failed, not that the file was
    // never there.
    let dll_path = std::fs::canonicalize(dll).map_err(|_| InjectError::DllNotFound {
        path: dll.to_path_buf(),
    })?;

    let pid = match target {
        Target::Pid(pid) => *pid,
        Target::Name(name) => process::find_by_name(name)?,
    };

    let handle = process::open(pid)?;

    let target_bitness = process::bitness(&handle)?;
    let injector = injector_bitness();
    if target_bitness != injector {
        return Err(InjectError::ArchitectureMismatch {
            target: target_bitness,
            injector,
        });
    }

    // Refuse a second copy. Loading the payload twice would run its DllMain
    // again and leave two hooks fighting over the same Present, so a payload
    // that is already present is a refusal, not a retry.
    if process::module_is_loaded(pid, &dll_path)? {
        return Err(InjectError::AlreadyInjected { pid });
    }

    // Create the readiness event before injecting, so it exists by the time the
    // payload tries to signal it.
    let readiness = readiness::ReadinessEvent::create(pid)?;

    let wide_path = to_wide_null(&dll_path);
    remote::load_library_in(&handle, pid, &wide_path, options.readiness_timeout)?;

    if !readiness.wait(options.readiness_timeout) {
        return Err(InjectError::PayloadNotReady {
            waited: options.readiness_timeout,
        });
    }

    Ok(Injected {
        pid,
        readiness_confirmed: true,
    })
}

/// The default marker file path the kill switch looks for, exposed so a caller
/// (or its documentation) can tell the user exactly which file to create.
pub fn kill_switch_marker_path() -> PathBuf {
    marker_path()
}

/// Encode a path as a null terminated UTF-16 string for the wide Win32 calls
/// that consume it.
fn to_wide_null(path: &Path) -> Vec<u16> {
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn injecting_a_dll_that_is_not_on_disk_reports_it_as_missing() {
        // This is checked before the target is touched, so it does not need a
        // running process to exercise.
        let missing = Path::new(r"C:\this\path\does\not\exist\stasis-payload.dll");
        let err = inject(
            &Target::Pid(std::process::id()),
            missing,
            &InjectOptions::default(),
        )
        .expect_err("a missing DLL must fail");
        assert!(
            matches!(err, InjectError::DllNotFound { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn injecting_by_a_name_that_is_not_running_reports_the_target_not_found() {
        // A real DLL path is used so the search reaches the name lookup rather
        // than stopping at the missing-file check. The crate's own compiled
        // library is guaranteed to be on disk while the test runs.
        let dll = compiled_self();
        let err = inject(
            &Target::Name("stasis-no-such-process-xyz.exe".into()),
            &dll,
            &InjectOptions::default(),
        )
        .expect_err("an absent process must fail");
        assert!(
            matches!(err, InjectError::TargetNotFound { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn the_injector_reports_its_own_architecture() {
        // Not much of an assertion on its own, but it pins the compile-time
        // decision so a bad edit to the cfg is caught.
        let bitness = injector_bitness();
        assert!(matches!(bitness, Bitness::X64 | Bitness::X86));
    }

    // A path to a file that certainly exists on disk during the test: the test
    // binary itself. Used where a test needs to get past the DLL-existence
    // check to reach a later failure.
    fn compiled_self() -> PathBuf {
        std::env::current_exe().expect("the test binary has a path")
    }
}
