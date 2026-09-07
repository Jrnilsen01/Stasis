//! Finding and inspecting the target process, without reading its memory for
//! anything but the checks injection genuinely needs.
//!
//! Everything here is read-only reconnaissance: which process has a given name,
//! whether it is 32 or 64 bit, and whether the payload is already loaded in it.
//! None of it reads game state, and it is kept in its own module so an auditor
//! can see the full extent of what Stasis looks at before it injects.

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::Path;

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Module32FirstW, Module32NextW, Process32FirstW, Process32NextW,
    MODULEENTRY32W, PROCESSENTRY32W, TH32CS_SNAPMODULE, TH32CS_SNAPMODULE32, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::SystemInformation::{
    GetNativeSystemInfo, PROCESSOR_ARCHITECTURE_AMD64, PROCESSOR_ARCHITECTURE_INTEL, SYSTEM_INFO,
};
use windows::Win32::System::Threading::{
    IsWow64Process, OpenProcess, PROCESS_ACCESS_RIGHTS, PROCESS_CREATE_THREAD,
    PROCESS_QUERY_INFORMATION, PROCESS_VM_OPERATION, PROCESS_VM_READ, PROCESS_VM_WRITE,
};

use crate::error::{Bitness, InjectError};

/// A process handle that closes itself when it goes out of scope. Injection
/// takes several fallible steps after the handle is opened, and a raw handle
/// leaks on every early return between them. Tying the close to the scope means
/// no failure path can forget it.
pub struct ProcessHandle(pub HANDLE);

impl Drop for ProcessHandle {
    fn drop(&mut self) {
        // The handle is valid whenever this type exists, because it is only
        // constructed from a successful OpenProcess.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

/// A snapshot handle from the ToolHelp API, closed on scope exit for the same
/// reason as [`ProcessHandle`].
struct Snapshot(HANDLE);

impl Drop for Snapshot {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

/// Find the process id of a running process by its executable name, matched
/// case-insensitively (Windows file names are). Returns the first match, or
/// `TargetNotFound` if nothing matches.
///
/// Note that two processes can share a name. This returns the first the OS
/// reports, which is enough for the harness and for the common single-instance
/// game, but the caller that needs a specific instance should inject by pid.
pub fn find_by_name(name: &str) -> Result<u32, InjectError> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }.map_err(|e| {
        InjectError::System {
            context: "Could not enumerate running processes",
            code: win_code(&e),
        }
    })?;
    let snapshot = Snapshot(snapshot);

    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };

    // Process32FirstW fails only when the snapshot is empty, which cannot
    // happen here because the calling process is always in it.
    if unsafe { Process32FirstW(snapshot.0, &mut entry) }.is_ok() {
        loop {
            if wide_eq_ignore_case(&entry.szExeFile, name) {
                return Ok(entry.th32ProcessID);
            }
            if unsafe { Process32NextW(snapshot.0, &mut entry) }.is_err() {
                break;
            }
        }
    }

    Err(InjectError::TargetNotFound {
        process: name.to_string(),
    })
}

/// Open the target with exactly the rights injection needs and no more:
/// create a thread in it, operate on and write to its memory, and read enough
/// to query its architecture. It deliberately does not ask for
/// `PROCESS_ALL_ACCESS`, both because it does not need it and because the
/// smaller request is less likely to be denied and easier to justify to
/// anyone reviewing what Stasis does to a game.
pub fn open(pid: u32) -> Result<ProcessHandle, InjectError> {
    let rights: PROCESS_ACCESS_RIGHTS = PROCESS_CREATE_THREAD
        | PROCESS_QUERY_INFORMATION
        | PROCESS_VM_OPERATION
        | PROCESS_VM_WRITE
        | PROCESS_VM_READ;

    let handle = unsafe { OpenProcess(rights, false, pid) };
    match handle {
        Ok(handle) => Ok(ProcessHandle(handle)),
        Err(e) => Err(classify_open_failure(pid, win_code(&e))),
    }
}

/// Turn a raw OpenProcess error code into the specific failure it represents.
/// Access denied and "the process is gone" are the two cases a user can act on,
/// so they get their own variants; anything else is surfaced as a system error
/// rather than being mislabelled as one of them.
fn classify_open_failure(pid: u32, code: u32) -> InjectError {
    const ERROR_ACCESS_DENIED: u32 = 5;
    const ERROR_INVALID_PARAMETER: u32 = 87;
    match code {
        ERROR_ACCESS_DENIED => InjectError::AccessDenied { pid },
        // OpenProcess reports a pid that no longer exists as an invalid
        // parameter. From the caller's point of view the process was there a
        // moment ago and has since exited, which is the honest thing to say.
        ERROR_INVALID_PARAMETER => InjectError::TargetExited,
        _ => InjectError::System {
            context: "Could not open the target process",
            code,
        },
    }
}

/// Determine whether the target is a 32 or 64 bit process.
///
/// On a 64 bit Windows a 32 bit process runs under WOW64, and `IsWow64Process`
/// reports exactly that. A process that is not under WOW64 is running natively,
/// so it matches the width of the OS, which is read from the system info rather
/// than assumed. This is what lets the injector refuse an impossible cross
/// architecture load with a clear message instead of a mysterious failure deep
/// inside the remote thread.
pub fn bitness(handle: &ProcessHandle) -> Result<Bitness, InjectError> {
    let mut is_wow64 = windows::Win32::Foundation::BOOL(0);
    unsafe { IsWow64Process(handle.0, &mut is_wow64) }.map_err(|e| InjectError::System {
        context: "Could not query the target's architecture",
        code: win_code(&e),
    })?;

    if is_wow64.as_bool() {
        // Only a 32 bit process on a 64 bit OS is under WOW64.
        return Ok(Bitness::X86);
    }

    Ok(native_os_bitness())
}

/// The bit width of the running injector, decided at compile time. This is one
/// side of the architecture comparison; the target is the other.
pub const fn injector_bitness() -> Bitness {
    if cfg!(target_pointer_width = "64") {
        Bitness::X64
    } else {
        Bitness::X86
    }
}

/// The bit width Windows itself is running as, for the native (non-WOW64) case.
fn native_os_bitness() -> Bitness {
    let mut info = SYSTEM_INFO::default();
    unsafe { GetNativeSystemInfo(&mut info) };
    // SAFETY of the union access: GetNativeSystemInfo always fills the named
    // fields of the anonymous struct, which is what we read.
    let arch = unsafe { info.Anonymous.Anonymous.wProcessorArchitecture };
    if arch == PROCESSOR_ARCHITECTURE_INTEL {
        Bitness::X86
    } else if arch == PROCESSOR_ARCHITECTURE_AMD64 {
        Bitness::X64
    } else {
        // ARM64 and anything newer host 64 bit user processes here. Treating an
        // unrecognised native architecture as 64 bit is the safe default on
        // current Windows, where 32 bit only ever appears as WOW64, which was
        // already handled above.
        Bitness::X64
    }
}

/// Whether a module with the given file name is already loaded in the target.
/// This is how a second injection is prevented: the payload's own file name is
/// looked for among the target's loaded modules, and finding it means the DLL
/// is already there.
pub fn module_is_loaded(pid: u32, dll: &Path) -> Result<bool, InjectError> {
    let Some(file_name) = dll.file_name().and_then(|n| n.to_str()) else {
        // A path with no file name cannot match any module, so nothing is
        // loaded under it. The DLL-not-found check upstream is what catches a
        // path this malformed as an error.
        return Ok(false);
    };

    // TH32CS_SNAPMODULE32 is included so that, on a 64 bit host inspecting a
    // WOW64 target, the 32 bit modules are listed too. Without it the check
    // could miss a payload that is genuinely loaded.
    let snapshot =
        unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, pid) }.map_err(
            |e| InjectError::System {
                context: "Could not list the target's loaded modules",
                code: win_code(&e),
            },
        )?;
    let snapshot = Snapshot(snapshot);

    let mut entry = MODULEENTRY32W {
        dwSize: std::mem::size_of::<MODULEENTRY32W>() as u32,
        ..Default::default()
    };

    if unsafe { Module32FirstW(snapshot.0, &mut entry) }.is_ok() {
        loop {
            if wide_eq_ignore_case(&entry.szModule, file_name) {
                return Ok(true);
            }
            if unsafe { Module32NextW(snapshot.0, &mut entry) }.is_err() {
                break;
            }
        }
    }

    Ok(false)
}

/// Compare a null-terminated wide buffer from a Win32 struct against a Rust
/// string, ignoring case the way the Windows file system does.
fn wide_eq_ignore_case(buffer: &[u16], name: &str) -> bool {
    let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    let text = OsString::from_wide(&buffer[..end]);
    match text.to_str() {
        Some(text) => text.eq_ignore_ascii_case(name),
        None => false,
    }
}

/// Extract the raw OS error number from a `windows` error so it can be stored in
/// a `System` variant. The `windows` crate carries the value as an HRESULT; the
/// low 16 bits are the original Win32 code for the FACILITY_WIN32 errors these
/// APIs return.
pub fn win_code(e: &windows::core::Error) -> u32 {
    (e.code().0 as u32) & 0xffff
}
