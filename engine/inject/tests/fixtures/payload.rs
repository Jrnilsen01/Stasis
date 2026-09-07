// A stand-in overlay payload, used only by this crate's tests.
//
// It is not part of the workspace. The integration tests compile it directly
// with rustc into a cdylib, so a real DLL exists to inject into the harness
// without waiting on the overlay crate and without ever building something a
// person might run against a game. It has no dependency beyond std and a
// handful of kernel32 functions declared by hand, which is what lets bare rustc
// build it.
//
// What it does on attach, and nothing more: honour the kill switch, write one
// line to a file so a test can prove it ran, and set the readiness event so the
// injector's handshake completes. Compile it with `--cfg stasis_signal` for the
// signalling build the success tests want, and without it for a silent build
// that loads cleanly but never signals, which is what the PayloadNotReady test
// needs.
//
// The work is kept tiny on purpose. DllMain runs under the Windows loader lock,
// where anything that triggers another module to load can deadlock, so a real
// payload would hand off to its own thread. A test payload doing one file write
// and one SetEvent stays within what is safe to do there.

use std::ffi::c_void;
use std::io::Write;
use std::path::PathBuf;

// The few kernel32 calls the readiness signal needs. kernel32 is always loaded,
// so these resolve without us loading anything, which keeps DllMain honest
// about the loader lock.
extern "system" {
    fn GetCurrentProcessId() -> u32;
    fn OpenEventW(access: u32, inherit: i32, name: *const u16) -> *mut c_void;
    fn SetEvent(handle: *mut c_void) -> i32;
    fn CloseHandle(handle: *mut c_void) -> i32;
}

const DLL_PROCESS_ATTACH: u32 = 1;
const EVENT_MODIFY_STATE: u32 = 0x0002;

#[no_mangle]
pub extern "system" fn DllMain(_module: *mut c_void, reason: u32, _reserved: *mut c_void) -> i32 {
    if reason == DLL_PROCESS_ATTACH {
        if kill_switch_engaged() {
            // Refuse to load. This is the payload's own copy of the check the
            // injector already made, so a DLL that somehow reached the process
            // still declines when the switch is set.
            return 0;
        }
        write_marker();
        if cfg!(stasis_signal) {
            signal_ready();
        }
    }
    1
}

// The same two markers the injector's killswitch module checks, read the same
// way, because the payload has to agree with it independently.
fn kill_switch_engaged() -> bool {
    if let Ok(value) = std::env::var("STASIS_DISABLE") {
        if !value.is_empty() {
            return true;
        }
    }
    let marker = match std::env::var_os("STASIS_DISABLE_FILE") {
        Some(path) if !path.is_empty() => PathBuf::from(path),
        _ => std::env::temp_dir().join("stasis-disable"),
    };
    marker.exists()
}

// Append one line to the log file so a test can assert the payload actually ran
// inside the target. The path comes from STASIS_PAYLOAD_LOG, which the test
// sets on the harness process before launching it; without it, a fixed temp
// file is used so the payload never fails for lack of a destination.
fn write_marker() {
    let path = match std::env::var_os("STASIS_PAYLOAD_LOG") {
        Some(path) if !path.is_empty() => PathBuf::from(path),
        _ => std::env::temp_dir().join("stasis-payload-loaded.txt"),
    };
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let pid = unsafe { GetCurrentProcessId() };
        let _ = writeln!(file, "stasis test payload attached to pid {pid}");
    }
}

// Open the readiness event the injector created and set it. The name is built
// from this process's own id, the same formula the injector uses, so the two
// meet without any value being passed between them.
fn signal_ready() {
    let pid = unsafe { GetCurrentProcessId() };
    let name = format!("Local\\stasis-payload-ready-{pid}");
    let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let handle = OpenEventW(EVENT_MODIFY_STATE, 0, wide.as_ptr());
        if !handle.is_null() {
            SetEvent(handle);
            CloseHandle(handle);
        }
    }
}
