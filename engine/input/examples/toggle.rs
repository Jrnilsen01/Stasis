//! Drives the state machine from a real hotkey, and prints every transition.
//!
//! This is the part of the crate that cannot be unit tested, because it ends at
//! a human pressing a key. Start the harness first (`cargo run -p
//! stasis-harness`) so there is a window presenting frames to toggle over, then
//! run this and press the combination.
//!
//! Standing in for the overlay crate, this example confirms its own acquires
//! and releases after a short delay. That is the only thing here that is not
//! real, and `--hang` turns it off so the deadline and the escalation can be
//! watched instead.
//!
//! ```text
//! cargo run -p stasis-input --example toggle
//! cargo run -p stasis-input --example toggle -- --self-post
//! cargo run -p stasis-input --example toggle -- --hang
//! cargo run -p stasis-input --example toggle -- "Ctrl+Alt+Shift+F7"
//! ```
//!
//! `--self-post` posts `WM_HOTKEY` into this process's own message queue so the
//! pump and the machine can be checked without a keypress. It is not
//! synthesised input: nothing is sent to the input queue and nothing reaches
//! another process. Actually delivering a keystroke is what a human doing the
//! run is for.

use std::time::Instant;

use stasis_input::{
    Capture, Effect, Hotkey, HotkeyRegistration, InputEvent, InputState, Machine, Millis,
    Transition, DEFAULT_HOTKEY,
};
use windows::core::w;
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, PeekMessageW, PostThreadMessageW, MSG, PM_REMOVE, WM_HOTKEY,
};

/// How long the stand-in overlay takes to answer, so the transient states are
/// visible in the output rather than passed through in the same iteration.
const CONFIRM_DELAY_MS: Millis = 120;

/// When `--self-post` is given, the second press comes this long after the
/// first: long enough for the acquire to have settled, so the run shows a full
/// capture and release rather than the double-press cancellation.
const SELF_POST_GAP_MS: Millis = 600;

struct Options {
    hotkey: Hotkey,
    self_post: bool,
    hang: bool,
}

fn parse_options() -> Result<Options, String> {
    let mut options = Options {
        hotkey: DEFAULT_HOTKEY,
        self_post: false,
        hang: false,
    };
    for argument in std::env::args().skip(1) {
        match argument.as_str() {
            "--self-post" => options.self_post = true,
            "--hang" => options.hang = true,
            other => {
                options.hotkey = other
                    .parse()
                    .map_err(|reason| format!("{other:?} is not a hotkey: {reason}"))?
            }
        }
    }
    Ok(options)
}

fn main() -> Result<(), String> {
    let options = parse_options()?;

    let harness = unsafe { FindWindowW(w!("StasisHarnessWindow"), None) }.ok();
    match harness {
        Some(hwnd) => println!("harness window  : {:?}", hwnd.0),
        None => println!(
            "harness window  : not found. Start it with `cargo run -p stasis-harness` so there is \
             something presenting frames to toggle over."
        ),
    }

    let registration = HotkeyRegistration::register(options.hotkey).map_err(|error| {
        // The failure the requirement is about. The message names the
        // combination and says it is taken, so the user is not left wondering
        // whether the overlay is broken.
        format!("{error}")
    })?;
    println!(
        "hotkey          : {} (id {})",
        registration.hotkey(),
        registration.id()
    );
    if options.hang {
        println!("overlay         : hanging on purpose, no confirmations will be sent");
    }
    println!("press it. Ctrl+C to stop.\n");

    let start = Instant::now();
    let now = || start.elapsed().as_millis() as Millis;

    let mut machine = Machine::new(now());
    let mut pending: Option<(InputEvent, Millis)> = None;
    let mut presses_posted: u32 = 0;
    let mut ever_held = false;

    loop {
        let at = now();
        let mut transitions: Vec<Transition> = Vec::new();

        if options.self_post
            && presses_posted < 2
            && at >= Millis::from(presses_posted) * SELF_POST_GAP_MS
        {
            post_hotkey_to_self(registration.id());
            presses_posted += 1;
        }

        let mut message = MSG::default();
        while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
            if message.message == WM_HOTKEY && registration.matches(message.wParam.0) {
                transitions.push(machine.on(InputEvent::HotkeyPressed, at));
            }
        }

        // Whatever the overlay owes, answered on a timer. In the real engine
        // this comes back from the overlay crate instead.
        if let Some((event, due)) = pending {
            if at >= due {
                pending = None;
                transitions.push(machine.on(event, at));
            }
        }

        transitions.push(machine.on(InputEvent::Tick, at));

        let mut tear_down = false;
        for transition in transitions {
            report(transition, &machine, at);
            match transition.effect {
                Effect::Acquire if !options.hang => {
                    pending = Some((InputEvent::CaptureConfirmed, at + CONFIRM_DELAY_MS))
                }
                Effect::Release if !options.hang => {
                    pending = Some((InputEvent::ReleaseConfirmed, at + CONFIRM_DELAY_MS))
                }
                Effect::ForceRelease => tear_down = true,
                _ => {}
            }
        }

        // The teardown is not something to wait on. It is the supervisor
        // reaching past the overlay, and it either happened or the supervisor
        // is going down with the answer, so it is answered in the same
        // iteration rather than scheduled like a confirmation.
        if tear_down {
            println!("         supervisor tears capture down without the overlay");
            pending = None;
            report(machine.on(InputEvent::ForcedRelease, at), &machine, at);
        }

        ever_held |= machine.holds_input();
        if options.self_post
            && presses_posted == 2
            && ever_held
            && pending.is_none()
            && machine.state() == InputState::Released
        {
            println!("\ninput is back with the game. done.");
            return Ok(());
        }

        std::thread::sleep(std::time::Duration::from_millis(8));
    }
}

/// Posts `WM_HOTKEY` to this thread's own queue.
///
/// Not input. `PostThreadMessage` puts a message in this process's queue and
/// touches nothing outside it, which is what makes it usable for checking the
/// pump without building the input-synthesis capability this project has
/// promised not to build.
fn post_hotkey_to_self(id: i32) {
    let thread = unsafe { GetCurrentThreadId() };
    let posted = unsafe { PostThreadMessageW(thread, WM_HOTKEY, WPARAM(id as usize), LPARAM(0)) };
    if let Err(error) = posted {
        eprintln!("could not post to this thread's own queue: {error}");
    }
}

fn report(transition: Transition, machine: &Machine, at: Millis) {
    if !transition.changed() && transition.effect == Effect::None {
        return;
    }
    let Capture {
        swallow_input,
        expires_at,
    } = machine.capture(at);
    println!(
        "[{at:>6}ms] {:?} -> {:?}  effect {:?}  swallow {swallow_input} until {expires_at}ms",
        transition.from, transition.to, transition.effect
    );
}
