//! The readiness handshake between the injector and the payload.
//!
//! Getting a DLL to load is not the same as the overlay working. `LoadLibraryW`
//! can return a valid handle for a DLL whose initialisation then fails halfway,
//! and it returns one just as happily for a DLL that is not a Stasis payload at
//! all. So the injector does not trust the load result alone. Before injecting,
//! it creates a named event, and the payload sets that event from inside its
//! own `DllMain` once it is genuinely up. If the event never fires, the
//! injector reports `PayloadNotReady` rather than claiming a success it cannot
//! see.
//!
//! The name is derived from the target's process id, which both sides know: the
//! injector because it chose the target, the payload because it can ask for its
//! own. This means no configuration has to be passed between the two processes
//! for the handshake to line up.

use std::time::Duration;

use windows::core::HSTRING;
use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject};

use crate::error::InjectError;
use crate::process::win_code;

/// Build the readiness event name for a given target pid. Kept as one function
/// so the injector and the payload cannot drift apart on the naming; the
/// payload fixture documents that it must format the name the same way. The
/// `Local\` prefix scopes the event to the current session, which is where both
/// processes live.
pub fn event_name(pid: u32) -> String {
    format!("Local\\stasis-payload-ready-{pid}")
}

/// A readiness event handle, closed on scope exit.
pub struct ReadinessEvent {
    handle: HANDLE,
}

impl Drop for ReadinessEvent {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}

impl ReadinessEvent {
    /// Create the event before injecting, so it already exists when the payload
    /// opens it by name. It is manual reset and starts unset: manual reset so
    /// that a payload which signals slightly before the injector waits is not
    /// missed, and unset so that waiting genuinely blocks until the payload
    /// acts.
    pub fn create(pid: u32) -> Result<Self, InjectError> {
        let name = HSTRING::from(event_name(pid));
        let handle =
            unsafe { CreateEventW(None, true, false, &name) }.map_err(|e| InjectError::System {
                context: "Could not create the readiness event",
                code: win_code(&e),
            })?;
        Ok(ReadinessEvent { handle })
    }

    /// Wait for the payload to signal readiness, up to `timeout`. Returns
    /// whether it signalled in time; the caller turns a `false` into
    /// `PayloadNotReady`.
    pub fn wait(&self, timeout: Duration) -> bool {
        let result = unsafe { WaitForSingleObject(self.handle, timeout.as_millis() as u32) };
        result == WAIT_OBJECT_0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_event_name_is_derived_from_the_pid_so_both_sides_agree() {
        // The payload computes this same string from its own process id, so if
        // this format ever changes the fixture in tests must change with it.
        assert_eq!(event_name(4321), r"Local\stasis-payload-ready-4321");
    }
}
