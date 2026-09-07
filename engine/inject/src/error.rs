//! The failure taxonomy for injection.
//!
//! Every way an injection can fail gets its own variant here, because the user
//! needs to be told which one happened. "Injection failed" is not an answer a
//! person can act on. "The target is 64 bit but Stasis is 32 bit" is. Each
//! variant carries the specific facts its sentence needs, and the `Display`
//! impl turns it straight into that sentence for the UI.

use std::fmt;
use std::path::PathBuf;
use std::time::Duration;

/// The bit width of a process image. Injection can only cross from one process
/// to another when the two agree, so this is compared, not just reported.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bitness {
    X86,
    X64,
}

impl fmt::Display for Bitness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Bitness::X86 => write!(f, "32-bit"),
            Bitness::X64 => write!(f, "64-bit"),
        }
    }
}

/// Why the kill switch refused an injection. Carried so the message can name
/// the exact marker the user has to remove to undo it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum KillSwitchReason {
    /// The `STASIS_DISABLE` environment variable was set to a non-empty value.
    EnvironmentVariable,
    /// A disable marker file was present on disk at the given path.
    MarkerFile(PathBuf),
}

impl fmt::Display for KillSwitchReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KillSwitchReason::EnvironmentVariable => {
                write!(f, "the STASIS_DISABLE environment variable is set")
            }
            KillSwitchReason::MarkerFile(path) => {
                write!(f, "the marker file {} exists", path.display())
            }
        }
    }
}

/// Every distinct reason an injection can fail.
///
/// This is deliberately not a `String`. The UI has to turn a failure into a
/// sentence a user can act on, and a stringly-typed error forces it to parse
/// prose to decide whether, for example, to offer an "elevate and retry"
/// button (access denied) or explain that a 32 bit game cannot be supported by
/// a 64 bit build (architecture mismatch). Those are different sentences with
/// different next steps, so they are different variants.
#[derive(Debug)]
pub enum InjectError {
    /// No running process matched the name that was searched for.
    TargetNotFound { process: String },

    /// The process was found, but it exited before the payload reached it. This
    /// is a race rather than a mistake: nothing the user did was wrong, the
    /// game just closed at an unlucky moment.
    TargetExited,

    /// The process refused to open with the access injection needs. Usually the
    /// target runs at a higher integrity level than Stasis.
    AccessDenied { pid: u32 },

    /// The target and the injector are different bit widths. A 64 bit process
    /// cannot host a 32 bit DLL and the reverse, so this can never succeed and
    /// is reported rather than attempted.
    ArchitectureMismatch { target: Bitness, injector: Bitness },

    /// The payload DLL was not present on disk at the path given.
    DllNotFound { path: PathBuf },

    /// The DLL reached the target but `LoadLibraryW` refused it, so it never
    /// ran. A missing dependency or a corrupt image lands here.
    DllLoadFailed { pid: u32 },

    /// The DLL loaded, but never signalled that it was ready within the
    /// timeout. Either it hung during initialisation, or the thing that was
    /// injected is not a Stasis payload and so does not speak the readiness
    /// protocol at all.
    PayloadNotReady { waited: Duration },

    /// The payload is already loaded in the target. A second injection would
    /// load a second copy, so it is refused instead.
    AlreadyInjected { pid: u32 },

    /// Injection was disabled by the kill switch before anything was attempted.
    /// This is a chosen state, not a fault, and is reported plainly so the user
    /// who set it is not left wondering why nothing happens.
    KillSwitchEngaged { reason: KillSwitchReason },

    /// A Win32 call failed in a way that is not one of the cases above. The
    /// context says which operation, and the code is the raw OS error, so an
    /// unexpected failure is still traceable rather than swallowed. The absence
    /// of a specific variant is reported honestly rather than guessed at.
    System { context: &'static str, code: u32 },
}

impl fmt::Display for InjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InjectError::TargetNotFound { process } => {
                write!(f, "No running process named \"{process}\" was found.")
            }
            InjectError::TargetExited => write!(
                f,
                "The target process exited before the payload could be loaded."
            ),
            InjectError::AccessDenied { pid } => write!(
                f,
                "Access to process {pid} was denied. Stasis may need to run \
                 with the same privileges as the target."
            ),
            InjectError::ArchitectureMismatch { target, injector } => write!(
                f,
                "Architecture mismatch: the target is {target} but Stasis is \
                 {injector}. A {injector} build cannot inject into a {target} \
                 process."
            ),
            InjectError::DllNotFound { path } => {
                write!(f, "The payload was not found at {}.", path.display())
            }
            InjectError::DllLoadFailed { pid } => write!(
                f,
                "The payload was copied into process {pid} but failed to load."
            ),
            InjectError::PayloadNotReady { waited } => write!(
                f,
                "The payload loaded but did not report readiness within {}.",
                format_duration(*waited)
            ),
            InjectError::AlreadyInjected { pid } => write!(
                f,
                "The payload is already loaded in process {pid}. Stasis will \
                 not inject a second copy."
            ),
            InjectError::KillSwitchEngaged { reason } => write!(
                f,
                "Injection is turned off by the Stasis kill switch, because \
                 {reason}. Clear it to allow injection again."
            ),
            InjectError::System { context, code } => {
                write!(f, "{context} (Windows error {code}).")
            }
        }
    }
}

impl std::error::Error for InjectError {}

/// Render a duration the way the readiness message wants to read it: whole
/// seconds when it divides evenly, so a five second timeout says "5 seconds"
/// rather than "5s" or "5.000s".
fn format_duration(d: Duration) -> String {
    let ms = d.as_millis();
    if ms.is_multiple_of(1000) {
        let secs = ms / 1000;
        let unit = if secs == 1 { "second" } else { "seconds" };
        format!("{secs} {unit}")
    } else {
        format!("{ms} ms")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_error_renders_a_sentence_that_ends_with_a_full_stop() {
        // The UI shows these verbatim, so a variant that renders without
        // terminating punctuation would read as a truncated fragment.
        let cases = [
            InjectError::TargetNotFound {
                process: "game.exe".into(),
            },
            InjectError::TargetExited,
            InjectError::AccessDenied { pid: 1234 },
            InjectError::ArchitectureMismatch {
                target: Bitness::X86,
                injector: Bitness::X64,
            },
            InjectError::DllNotFound {
                path: PathBuf::from(r"C:\missing.dll"),
            },
            InjectError::DllLoadFailed { pid: 1234 },
            InjectError::PayloadNotReady {
                waited: Duration::from_secs(5),
            },
            InjectError::AlreadyInjected { pid: 1234 },
            InjectError::KillSwitchEngaged {
                reason: KillSwitchReason::EnvironmentVariable,
            },
            InjectError::System {
                context: "OpenProcess failed",
                code: 5,
            },
        ];
        for case in cases {
            let sentence = case.to_string();
            assert!(
                sentence.ends_with('.'),
                "variant {case:?} rendered without a full stop: {sentence:?}"
            );
        }
    }

    #[test]
    fn the_architecture_mismatch_sentence_names_both_sides() {
        // The whole value of this variant over a generic failure is that it can
        // tell the user which side is which, so both widths must appear.
        let sentence = InjectError::ArchitectureMismatch {
            target: Bitness::X86,
            injector: Bitness::X64,
        }
        .to_string();
        assert!(sentence.contains("target is 32-bit"));
        assert!(sentence.contains("Stasis is 64-bit"));
    }

    #[test]
    fn a_five_second_timeout_reads_as_whole_seconds() {
        let sentence = InjectError::PayloadNotReady {
            waited: Duration::from_secs(5),
        }
        .to_string();
        assert!(sentence.contains("5 seconds"), "got: {sentence}");
    }

    #[test]
    fn the_kill_switch_sentence_names_the_marker_file_so_it_can_be_removed() {
        let sentence = InjectError::KillSwitchEngaged {
            reason: KillSwitchReason::MarkerFile(PathBuf::from(r"C:\Temp\stasis-disable")),
        }
        .to_string();
        assert!(
            sentence.contains(r"C:\Temp\stasis-disable"),
            "got: {sentence}"
        );
    }
}
