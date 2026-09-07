//! Registration checked against a live Windows desktop, with the harness up.
//!
//! `RegisterHotKey` does not itself need the harness. The reason these tests
//! gate on it is that the situation being checked is a game-shaped process
//! presenting frames in the foreground while Stasis takes a global binding out
//! from under it, and that is the only thing anyone is allowed to point this
//! code at. Start it with `cargo run -p stasis-harness`; without it, every test
//! here reports that it skipped and passes.
//!
//! Nothing here presses a key. Synthesising input is a cheat capability and the
//! project has promised not to build one, so the parts of the path that need a
//! real keypress are exercised by a human running `examples/toggle.rs`.

use stasis_input::{Hotkey, HotkeyError, HotkeyRegistration, Key, Modifiers, DEFAULT_HOTKEY};
use windows::core::w;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::FindWindowW;

/// The class name the harness registers, which is also what the detector
/// matches on.
fn harness_window() -> Option<HWND> {
    unsafe { FindWindowW(w!("StasisHarnessWindow"), None) }.ok()
}

/// Returns the harness window, or prints why the test did nothing and returns
/// `None`.
///
/// A skip that says so beats a skip that looks like a pass, because the second
/// one lets the live coverage quietly reach zero.
fn require_harness(test: &str) -> Option<HWND> {
    match harness_window() {
        Some(hwnd) => Some(hwnd),
        None => {
            println!("{test}: skipped, the harness is not running (cargo run -p stasis-harness)");
            None
        }
    }
}

/// A binding for the tests to take and give back.
///
/// F21 has no key on any keyboard, so taking it desktop-wide for the length of
/// a test cannot interrupt anything the person running the suite is doing.
fn spare(n: u8) -> Hotkey {
    Hotkey::new(Modifiers::CTRL_SHIFT, Key::function(n).expect("F21 to F24"))
}

#[test]
fn the_default_hotkey_registers_against_a_live_desktop() {
    let Some(_) = require_harness("the_default_hotkey_registers_against_a_live_desktop") else {
        return;
    };

    match HotkeyRegistration::register(DEFAULT_HOTKEY) {
        Ok(registration) => {
            assert_eq!(registration.hotkey(), DEFAULT_HOTKEY);
            println!(
                "registered {} as id {}",
                registration.hotkey(),
                registration.id()
            );
        }
        Err(HotkeyError::AlreadyTaken(hotkey)) => {
            // Not a failure. Something on this machine holds the default, which
            // is the case rebinding exists for, and the message says which
            // combination rather than leaving the reader guessing.
            println!(
                "{hotkey} is taken on this machine: {}",
                HotkeyError::AlreadyTaken(hotkey)
            );
        }
        Err(other) => panic!("registering {DEFAULT_HOTKEY} failed unexpectedly: {other}"),
    }
}

#[test]
fn a_combination_already_held_is_reported_as_taken_and_names_itself() {
    // The honest-reporting path, checked against Windows rather than against a
    // constructed error value. The first registration is what makes the second
    // one fail, so this needs no other application to cooperate.
    let Some(_) =
        require_harness("a_combination_already_held_is_reported_as_taken_and_names_itself")
    else {
        return;
    };

    let hotkey = spare(21);
    let held = HotkeyRegistration::register(hotkey).expect("F21 should be free");

    let second = HotkeyRegistration::register(hotkey);
    match second {
        Err(HotkeyError::AlreadyTaken(reported)) => {
            assert_eq!(reported, hotkey);
            let message = HotkeyError::AlreadyTaken(reported).to_string();
            assert!(message.contains(&hotkey.to_string()));
            println!("windows reported: {message}");
        }
        Err(other) => panic!("expected the taken error, got {other}"),
        Ok(_) => panic!("windows registered {hotkey} twice, which it is documented not to do"),
    }

    drop(held);
}

#[test]
fn dropping_a_registration_gives_the_combination_back_to_the_desktop() {
    // The release path for the binding itself. If this leaked, a user who
    // rebound twice would have collected combinations nothing can use until
    // they restart.
    let Some(_) =
        require_harness("dropping_a_registration_gives_the_combination_back_to_the_desktop")
    else {
        return;
    };

    let hotkey = spare(22);
    let first = HotkeyRegistration::register(hotkey).expect("F22 should be free");
    assert!(
        HotkeyRegistration::register(hotkey).is_err(),
        "the combination should be held while the first registration is alive"
    );

    drop(first);
    let again = HotkeyRegistration::register(hotkey);
    assert!(
        again.is_ok(),
        "{hotkey} was not released when its registration was dropped"
    );
}

#[test]
fn a_rebind_that_fails_leaves_the_working_binding_in_place() {
    // The property that matters when the overlay is already open: a rebind onto
    // something taken must not take away the key that closes it.
    let Some(_) = require_harness("a_rebind_that_fails_leaves_the_working_binding_in_place") else {
        return;
    };

    let original = spare(23);
    let blocker_hotkey = spare(24);
    let mut registration = HotkeyRegistration::register(original).expect("F23 should be free");
    let blocker = HotkeyRegistration::register(blocker_hotkey).expect("F24 should be free");
    let id_before = registration.id();

    assert_eq!(
        registration.rebind(blocker_hotkey),
        Err(HotkeyError::AlreadyTaken(blocker_hotkey))
    );
    assert_eq!(registration.hotkey(), original);
    assert_eq!(registration.id(), id_before);

    drop(blocker);
    assert_eq!(registration.rebind(blocker_hotkey), Ok(()));
    assert_eq!(registration.hotkey(), blocker_hotkey);
    assert_ne!(registration.id(), id_before, "a rebind takes a fresh id");
}
