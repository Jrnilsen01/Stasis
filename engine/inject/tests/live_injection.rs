//! Live injection tests against the D3D11 harness.
//!
//! These are the only tests that inject a real DLL into a real process, and
//! that process is always `stasis-harness`, never a game. They are marked
//! `#[ignore]` so a plain `cargo test` passes without them, because they need
//! the harness binary built, a GPU that can create a D3D11 device, and a rustc
//! to compile the payload fixture. Run them with:
//!
//!   cargo build -p stasis-harness
//!   cargo test -p stasis-inject -- --ignored --test-threads=1
//!
//! Every test skips itself, rather than failing, when the harness binary or
//! rustc is missing, so running them on a machine that cannot host the harness
//! reports a skip instead of a false failure.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use stasis_inject::{inject, InjectError, InjectOptions, Target};

// --- test scaffolding ----------------------------------------------------

// The harness executable, if it has been built. Its absence is the signal to
// skip: these tests do not build it themselves, because building a GUI binary
// as a side effect of a unit test run is surprising.
fn harness_exe() -> Option<PathBuf> {
    let exe = workspace_target().join("debug").join("stasis-harness.exe");
    exe.exists().then_some(exe)
}

fn workspace_target() -> PathBuf {
    // The integration test runs with CARGO_MANIFEST_DIR set to this crate, and
    // the shared target directory sits one level up in the engine workspace.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("inject crate has a parent directory")
        .join("target")
}

fn rustc_available() -> bool {
    Command::new("rustc")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

// Compile the payload fixture into a cdylib and return its path. `signal`
// selects the build that sets the readiness event; without it, the payload
// loads but stays silent, which is the PayloadNotReady case.
fn compile_payload(out_dir: &Path, file_stem: &str, signal: bool) -> PathBuf {
    let src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("payload.rs");
    let out = out_dir.join(format!("{file_stem}.dll"));

    let mut cmd = Command::new("rustc");
    cmd.arg("--edition")
        .arg("2021")
        .arg("--crate-type")
        .arg("cdylib")
        .arg("--crate-name")
        .arg(file_stem.replace('-', "_"))
        .arg("-O")
        .arg("-o")
        .arg(&out)
        .arg(&src);
    if signal {
        cmd.arg("--cfg").arg("stasis_signal");
    }

    let status = cmd.status().expect("rustc runs");
    assert!(
        status.success(),
        "rustc failed to build the payload fixture"
    );
    assert!(out.exists(), "the payload DLL was not produced");
    out
}

// A running harness, killed on drop so a panicking test does not leave a window
// on screen.
struct Harness {
    child: Child,
    pid: u32,
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

// Launch the harness with a payload log path in its environment, and read back
// the pid it prints. Returns None when the harness cannot start or never
// reports a pid, which is how a machine with no usable GPU skips these tests.
fn spawn_harness(log_path: &Path) -> Option<Harness> {
    let exe = harness_exe()?;
    let mut child = Command::new(exe)
        .env("STASIS_PAYLOAD_LOG", log_path)
        .stdout(Stdio::piped())
        .spawn()
        .ok()?;

    let stdout = child.stdout.take()?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            if let Some(rest) = line.split_once("harness pid") {
                if let Some(pid) = parse_trailing_number(rest.1) {
                    let _ = tx.send(pid);
                    return;
                }
            }
        }
    });

    match rx.recv_timeout(Duration::from_secs(10)) {
        Ok(pid) => Some(Harness { child, pid }),
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            None
        }
    }
}

fn parse_trailing_number(text: &str) -> Option<u32> {
    text.trim()
        .trim_start_matches(':')
        .trim()
        .parse::<u32>()
        .ok()
}

// A macro so a skip reads clearly at each call site and returns from the test.
macro_rules! skip_unless_live {
    () => {{
        if harness_exe().is_none() {
            eprintln!("skipping: stasis-harness is not built");
            return;
        }
        if !rustc_available() {
            eprintln!("skipping: rustc is not available to build the payload");
            return;
        }
    }};
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("stasis-inject-live-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

// --- the tests -----------------------------------------------------------

#[test]
#[ignore = "needs the harness built and a GPU; run with --ignored"]
fn the_payload_loads_into_the_harness_and_the_harness_keeps_presenting() {
    skip_unless_live!();
    let dir = temp_dir("success");
    let log = dir.join("loaded.txt");
    let dll = compile_payload(&dir, "stasis_payload_ready", true);

    let Some(mut harness) = spawn_harness(&log) else {
        eprintln!("skipping: the harness could not start (no usable GPU?)");
        return;
    };

    let injected = inject(&Target::Pid(harness.pid), &dll, &InjectOptions::default())
        .expect("injection into the harness should succeed");
    assert_eq!(injected.pid, harness.pid);
    assert!(injected.readiness_confirmed);

    // The side effect proves the payload's DllMain actually ran inside the
    // harness, rather than the load merely being reported as successful.
    let logged = std::fs::read_to_string(&log).expect("payload wrote its log line");
    assert!(
        logged.contains(&format!("attached to pid {}", harness.pid)),
        "unexpected payload log: {logged:?}"
    );

    // And the harness is still alive afterwards, so injection did not take the
    // target down with it.
    assert!(
        harness.child.try_wait().expect("poll harness").is_none(),
        "the harness exited after injection"
    );
}

#[test]
#[ignore = "needs the harness built and a GPU; run with --ignored"]
fn a_second_injection_is_refused_because_the_payload_is_already_present() {
    skip_unless_live!();
    let dir = temp_dir("already");
    let log = dir.join("loaded.txt");
    let dll = compile_payload(&dir, "stasis_payload_ready", true);

    let Some(harness) = spawn_harness(&log) else {
        eprintln!("skipping: the harness could not start (no usable GPU?)");
        return;
    };

    inject(&Target::Pid(harness.pid), &dll, &InjectOptions::default())
        .expect("first injection should succeed");

    let err = inject(&Target::Pid(harness.pid), &dll, &InjectOptions::default())
        .expect_err("a second injection must be refused");
    assert!(
        matches!(err, InjectError::AlreadyInjected { .. }),
        "got {err:?}"
    );
}

#[test]
#[ignore = "needs the harness built and a GPU; run with --ignored"]
fn a_file_that_is_not_a_valid_dll_reports_a_load_failure() {
    skip_unless_live!();
    let dir = temp_dir("badfile");
    let log = dir.join("loaded.txt");

    // A text file with a .dll name exists on disk, so it passes the existence
    // check and reaches the target, where LoadLibraryW rejects it.
    let bogus = dir.join("not-really-a.dll");
    std::fs::write(&bogus, b"this is not a portable executable").expect("write bogus dll");

    let Some(harness) = spawn_harness(&log) else {
        eprintln!("skipping: the harness could not start (no usable GPU?)");
        return;
    };

    let err = inject(&Target::Pid(harness.pid), &bogus, &InjectOptions::default())
        .expect_err("loading a non-DLL must fail");
    assert!(
        matches!(err, InjectError::DllLoadFailed { .. }),
        "got {err:?}"
    );
}

#[test]
#[ignore = "needs the harness built and a GPU; run with --ignored"]
fn a_payload_that_never_signals_is_reported_as_not_ready() {
    skip_unless_live!();
    let dir = temp_dir("silent");
    let log = dir.join("loaded.txt");
    // The silent build loads cleanly but never sets the readiness event.
    let dll = compile_payload(&dir, "stasis_payload_silent", false);

    let Some(harness) = spawn_harness(&log) else {
        eprintln!("skipping: the harness could not start (no usable GPU?)");
        return;
    };

    // A short timeout keeps the test quick; the point is only that the wait
    // ends in the right error rather than a claimed success.
    let options = InjectOptions {
        readiness_timeout: Duration::from_secs(2),
    };
    let err = inject(&Target::Pid(harness.pid), &dll, &options)
        .expect_err("a silent payload must not be reported as ready");
    assert!(
        matches!(err, InjectError::PayloadNotReady { .. }),
        "got {err:?}"
    );
}
