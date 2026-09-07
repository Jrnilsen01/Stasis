//! The load itself: getting `LoadLibraryW` to run in the target with the
//! payload's path as its argument.
//!
//! This is the oldest and most boring DLL injection technique there is, and
//! that is the point. It uses only documented Win32 calls, does nothing to hide
//! itself, and reads the way the Microsoft documentation for each call reads.
//! Anyone auditing whether Stasis behaves like a cheat should be able to follow
//! it end to end: allocate a buffer in the target, write the DLL path into it,
//! then start a thread whose entry point is the real `LoadLibraryW` and whose
//! argument is that buffer. Windows does the loading; we supply a path.

use std::ffi::c_void;
use std::time::Duration;

use windows::core::s;
use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows::Win32::System::Diagnostics::Debug::WriteProcessMemory;
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::System::Memory::{
    VirtualAllocEx, VirtualFreeEx, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE,
};
use windows::Win32::System::Threading::{
    CreateRemoteThread, GetExitCodeThread, WaitForSingleObject, LPTHREAD_START_ROUTINE,
};

use crate::error::InjectError;
use crate::process::{win_code, ProcessHandle};

// A remote allocation that frees itself in the target when it goes out of
// scope, so the DLL path buffer is not left behind in the game's address space
// if a later step fails.
struct RemoteBuffer<'a> {
    process: &'a ProcessHandle,
    address: *mut c_void,
}

impl Drop for RemoteBuffer<'_> {
    fn drop(&mut self) {
        // Size zero with MEM_RELEASE releases the whole reservation made by the
        // matching VirtualAllocEx, which is what the docs require.
        unsafe {
            let _ = VirtualFreeEx(self.process.0, self.address, 0, MEM_RELEASE);
        }
    }
}

// A thread handle for the remote thread, closed on scope exit.
struct ThreadHandle(HANDLE);

impl Drop for ThreadHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

/// Cause the target to load the DLL at `dll_path_utf16` (a null terminated wide
/// string) by running `LoadLibraryW` on a thread inside it.
///
/// Returns once the load has finished. A load that does not return within
/// `load_timeout` is treated as the payload hanging during initialisation and
/// reported as `PayloadNotReady`. A load that returns a null module handle is
/// reported as `DllLoadFailed`. On 64 bit Windows the thread exit code is only
/// the low 32 bits of the returned `HMODULE`, so it is trustworthy as a zero
/// versus non-zero signal but not as the handle itself; the real confirmation
/// that a Stasis payload ran comes from the readiness event, not from here.
pub fn load_library_in(
    process: &ProcessHandle,
    pid: u32,
    dll_path_utf16: &[u16],
    load_timeout: Duration,
) -> Result<(), InjectError> {
    let byte_len = std::mem::size_of_val(dll_path_utf16);

    // Reserve and commit a buffer in the target just large enough for the path.
    // It only ever holds a file path, so read/write with no execute is all the
    // protection it needs, and asking for no more keeps the footprint honest.
    let address = unsafe {
        VirtualAllocEx(
            process.0,
            None,
            byte_len,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        )
    };
    if address.is_null() {
        return Err(InjectError::System {
            context: "Could not allocate memory in the target process",
            code: last_error(),
        });
    }
    let buffer = RemoteBuffer { process, address };

    let mut written: usize = 0;
    unsafe {
        WriteProcessMemory(
            process.0,
            buffer.address,
            dll_path_utf16.as_ptr() as *const c_void,
            byte_len,
            Some(&mut written),
        )
    }
    .map_err(|e| InjectError::System {
        context: "Could not write the payload path into the target",
        code: win_code(&e),
    })?;

    // kernel32 is loaded at the same base address in every process in a session,
    // so the address of LoadLibraryW here is also its address in the target.
    // This is the assumption the whole technique rests on, and it holds because
    // kernel32 is one of the known DLLs Windows maps identically everywhere.
    let start_routine = load_library_address()?;

    let thread = unsafe {
        CreateRemoteThread(
            process.0,
            None,
            0,
            start_routine,
            Some(buffer.address),
            0,
            None,
        )
    }
    .map_err(|e| InjectError::System {
        context: "Could not start the loader thread in the target",
        code: win_code(&e),
    })?;
    let thread = ThreadHandle(thread);

    let waited = unsafe { WaitForSingleObject(thread.0, load_timeout.as_millis() as u32) };
    if waited == WAIT_TIMEOUT {
        return Err(InjectError::PayloadNotReady {
            waited: load_timeout,
        });
    }
    if waited != WAIT_OBJECT_0 {
        return Err(InjectError::System {
            context: "Waiting for the loader thread failed",
            code: last_error(),
        });
    }

    let mut exit_code: u32 = 0;
    unsafe { GetExitCodeThread(thread.0, &mut exit_code) }.map_err(|e| InjectError::System {
        context: "Could not read the loader thread result",
        code: win_code(&e),
    })?;

    // A zero HMODULE means LoadLibraryW returned NULL, so the DLL was rejected
    // and its DllMain never ran. See the note on truncation above for why this
    // is read only as a zero versus non-zero signal.
    if exit_code == 0 {
        return Err(InjectError::DllLoadFailed { pid });
    }

    Ok(())
}

/// Resolve `LoadLibraryW` as a thread start routine. The signatures line up:
/// `LoadLibraryW` takes one pointer and returns a pointer, which is exactly the
/// shape of a thread routine on Windows, so the address can stand in as the
/// thread's entry point with its argument being the path buffer.
fn load_library_address() -> Result<LPTHREAD_START_ROUTINE, InjectError> {
    let kernel32 = unsafe { GetModuleHandleW(windows::core::w!("kernel32.dll")) }.map_err(|e| {
        InjectError::System {
            context: "Could not locate kernel32 in this process",
            code: win_code(&e),
        }
    })?;

    let proc = unsafe { GetProcAddress(kernel32, s!("LoadLibraryW")) };
    match proc {
        // Transmuting one Win32 function pointer to another with a compatible
        // ABI is exactly what CreateRemoteThread expects here.
        Some(proc) => Ok(Some(unsafe {
            std::mem::transmute::<
                unsafe extern "system" fn() -> isize,
                unsafe extern "system" fn(*mut c_void) -> u32,
            >(proc)
        })),
        None => Err(InjectError::System {
            context: "Could not find LoadLibraryW in kernel32",
            code: last_error(),
        }),
    }
}

/// The last-error value for the calls above that report failure through a null
/// return or a zero handle rather than through a `windows` Result.
fn last_error() -> u32 {
    unsafe { windows::Win32::Foundation::GetLastError().0 }
}
