//! `RegisterHotKey`, wrapped so that a failure is reported rather than shrugged
//! off.
//!
//! The registration deliberately lives here, in the process outside the game,
//! and not in the hook. A hotkey registered by the supervisor is delivered by
//! Windows out of the raw input queue before the game's window procedure sees
//! the key, so a hook that has hung cannot eat the key that turns it off. That
//! is the whole reason the toggle is a global hotkey and not a key the overlay
//! reads for itself.

use std::sync::atomic::{AtomicI32, Ordering};

use windows::Win32::Foundation::{GetLastError, ERROR_HOTKEY_ALREADY_REGISTERED};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT,
    MOD_SHIFT, MOD_WIN,
};

use crate::hotkey::{Hotkey, HotkeyError};

/// Identifiers for hot keys registered by an application have to sit in
/// 0x0000 to 0xBFFF, and each rebind takes a fresh one so the old registration
/// can outlive the new one's failure.
static NEXT_ID: AtomicI32 = AtomicI32::new(1);

fn take_id() -> i32 {
    // Wrapping below the reserved range rather than saturating, because a
    // process that rebinds forty thousand times should keep working rather than
    // start failing on an identifier clash it cannot see.
    NEXT_ID
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| {
            Some(if id >= 0xBFFF { 1 } else { id + 1 })
        })
        .unwrap_or(1)
}

fn windows_modifiers(hotkey: Hotkey) -> HOT_KEY_MODIFIERS {
    let mut flags = HOT_KEY_MODIFIERS(0);
    if hotkey.modifiers.ctrl {
        flags |= MOD_CONTROL;
    }
    if hotkey.modifiers.alt {
        flags |= MOD_ALT;
    }
    if hotkey.modifiers.shift {
        flags |= MOD_SHIFT;
    }
    if hotkey.modifiers.win {
        flags |= MOD_WIN;
    }
    // Auto-repeat would turn a held chord into a stream of toggles, and the
    // machine would spend the whole time bouncing between acquiring and
    // releasing. The state machine survives that, but the player watching the
    // overlay strobe would not call it working.
    flags | MOD_NOREPEAT
}

/// A live hotkey registration, released when this value is dropped.
///
/// Owning the registration in a value rather than in a global means the release
/// happens on every path out, including a panic unwinding through the caller.
/// Windows also drops every registration a process holds when the process dies,
/// so a crash cannot leave a combination taken machine-wide.
#[derive(Debug)]
pub struct HotkeyRegistration {
    hotkey: Hotkey,
    id: i32,
    thread: u32,
}

impl HotkeyRegistration {
    /// Registers `hotkey` for the calling thread.
    ///
    /// With no window given, Windows posts `WM_HOTKEY` to the calling thread's
    /// own message queue, so the caller needs a message pump on this thread and
    /// nothing else. The registration must be dropped on the same thread.
    pub fn register(hotkey: Hotkey) -> Result<HotkeyRegistration, HotkeyError> {
        hotkey.check()?;
        let id = take_id();
        register_raw(hotkey, id)?;
        Ok(HotkeyRegistration {
            hotkey,
            id,
            thread: unsafe { GetCurrentThreadId() },
        })
    }

    /// The combination currently registered.
    pub fn hotkey(&self) -> Hotkey {
        self.hotkey
    }

    /// The identifier Windows sends in `WM_HOTKEY`'s `wParam`.
    pub fn id(&self) -> i32 {
        self.id
    }

    /// Whether a `WM_HOTKEY` message belongs to this registration.
    pub fn matches(&self, wparam: usize) -> bool {
        wparam == self.id as usize
    }

    /// Moves the registration to a different combination.
    ///
    /// The new combination is registered before the old one is given up, so a
    /// rebind onto something another application already holds leaves the
    /// working toggle in place and returns the reason. Losing the old binding
    /// on the way to a binding that turns out to be unavailable would leave the
    /// user with no way to close an overlay that is already open, which is the
    /// failure this crate is written against.
    pub fn rebind(&mut self, next: Hotkey) -> Result<(), HotkeyError> {
        if next == self.hotkey {
            return Ok(());
        }
        next.check()?;
        let next_id = take_id();
        register_raw(next, next_id)?;

        unregister_raw(self.id);
        self.hotkey = next;
        self.id = next_id;
        Ok(())
    }
}

impl Drop for HotkeyRegistration {
    fn drop(&mut self) {
        debug_assert_eq!(
            self.thread,
            unsafe { GetCurrentThreadId() },
            "a hotkey has to be unregistered on the thread that registered it, \
             or Windows keeps the combination until the process exits"
        );
        unregister_raw(self.id);
    }
}

fn register_raw(hotkey: Hotkey, id: i32) -> Result<(), HotkeyError> {
    let registered = unsafe {
        RegisterHotKey(
            None,
            id,
            windows_modifiers(hotkey),
            u32::from(hotkey.key.virtual_key()),
        )
    };
    if registered.is_ok() {
        return Ok(());
    }

    let code = unsafe { GetLastError() };
    let failure = if code == ERROR_HOTKEY_ALREADY_REGISTERED {
        HotkeyError::AlreadyTaken(hotkey)
    } else {
        HotkeyError::Rejected {
            hotkey,
            code: code.0,
        }
    };

    // Also logged, not only returned. A caller that shows this on screen and a
    // caller that swallows it both exist, and a bug report about an overlay
    // that "does nothing" is answered by this line being in the log file.
    log::warn!("{failure}");
    Err(failure)
}

fn unregister_raw(id: i32) {
    // Nothing useful to do with a failure here. The combination is either free
    // now or it was never ours, and Windows releases everything this process
    // holds when it exits either way.
    let _ = unsafe { UnregisterHotKey(None, id) };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hotkey::{Key, Modifiers, DEFAULT_HOTKEY};

    /// Tests run in parallel, so each one that actually registers something
    /// takes a combination of its own. F21 upward is chosen because no
    /// keyboard has those keys, so nothing on a developer machine is holding
    /// them and no test can steal a real binding for the length of its run.
    fn spare(n: u8) -> Hotkey {
        Hotkey::new(Modifiers::CTRL_SHIFT, Key::function(n).expect("F21 to F24"))
    }

    #[test]
    fn every_modifier_reaches_its_windows_flag() {
        let all = Hotkey::new(
            Modifiers {
                ctrl: true,
                alt: true,
                shift: true,
                win: true,
            },
            Key::F9,
        );
        let flags = windows_modifiers(all);
        for expected in [MOD_CONTROL, MOD_ALT, MOD_SHIFT, MOD_WIN] {
            assert_eq!(flags & expected, expected);
        }
    }

    #[test]
    fn a_registration_always_asks_windows_to_suppress_auto_repeat() {
        // Without this, holding the chord toggles the overlay on every repeat
        // the keyboard sends.
        let flags = windows_modifiers(DEFAULT_HOTKEY);
        assert_eq!(flags & MOD_NOREPEAT, MOD_NOREPEAT);
    }

    #[test]
    fn a_hotkey_with_one_modifier_does_not_carry_the_others() {
        let flags = windows_modifiers(Hotkey::new(Modifiers::CTRL_SHIFT, Key::F9));
        assert_eq!(flags & MOD_ALT, HOT_KEY_MODIFIERS(0));
        assert_eq!(flags & MOD_WIN, HOT_KEY_MODIFIERS(0));
    }

    #[test]
    fn identifiers_stay_inside_the_range_windows_reserves_for_applications() {
        for _ in 0..1_000 {
            let id = take_id();
            assert!((1..=0xBFFF).contains(&id), "{id} is outside the range");
        }
    }

    #[test]
    fn identifiers_are_not_handed_out_twice_in_a_row() {
        // A rebind registers the new combination while the old one is still
        // held, so reusing the identifier would collide with itself.
        assert_ne!(take_id(), take_id());
    }

    #[test]
    fn a_combination_with_no_modifier_is_refused_without_calling_windows() {
        // Checked here rather than only in `hotkey` because this is the path
        // that would otherwise reach `RegisterHotKey`, which accepts it.
        let bare = Hotkey::new(Modifiers::NONE, Key::F9);
        assert_eq!(
            HotkeyRegistration::register(bare).map(|_| ()),
            Err(HotkeyError::NoModifier(bare))
        );
    }

    #[test]
    fn a_reserved_combination_is_refused_without_calling_windows() {
        let reserved: Hotkey = "Ctrl+Alt+Delete".parse().expect("parses");
        assert_eq!(
            HotkeyRegistration::register(reserved).map(|_| ()),
            Err(HotkeyError::Reserved(reserved))
        );
    }

    #[test]
    fn rebinding_to_the_same_combination_does_nothing_rather_than_failing() {
        // The settings screen writes the binding on every save, so the no-op
        // case is the common one. Re-registering would fail against itself.
        let hotkey = spare(21);
        let Ok(mut registration) = HotkeyRegistration::register(hotkey) else {
            // Something else on this machine holds it. That is a real outcome
            // on a live desktop, not a test failure.
            return;
        };
        assert_eq!(registration.rebind(hotkey), Ok(()));
        assert_eq!(registration.hotkey(), hotkey);
    }

    #[test]
    fn a_rebind_onto_a_reserved_combination_leaves_the_old_binding_working() {
        let hotkey = spare(22);
        let Ok(mut registration) = HotkeyRegistration::register(hotkey) else {
            return;
        };
        let before = registration.id();

        let reserved: Hotkey = "Ctrl+Shift+Escape".parse().expect("parses");
        assert_eq!(
            registration.rebind(reserved),
            Err(HotkeyError::Reserved(reserved))
        );
        assert_eq!(registration.hotkey(), hotkey);
        assert_eq!(registration.id(), before);
    }

    #[test]
    fn a_hotkey_message_is_matched_by_its_identifier() {
        let Ok(registration) = HotkeyRegistration::register(spare(23)) else {
            return;
        };
        assert!(registration.matches(registration.id() as usize));
        assert!(!registration.matches(registration.id() as usize + 1));
    }
}
