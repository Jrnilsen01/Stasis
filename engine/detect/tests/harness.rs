//! Detection against the one target this project is allowed to touch.
//!
//! The harness in `engine/harness` presents D3D11 frames the way a game does.
//! No test in this repository may attach to, enumerate for the purpose of
//! attaching to, or otherwise touch a real game, because getting the owner's
//! accounts banned is the worst outcome this project has.
//!
//! Every test here skips when the harness is not running, so `cargo test`
//! passes on a machine that has never started it. Start it with
//! `cargo run -p stasis-harness` from `engine/` and run again to get the real
//! coverage, which is the only end to end evidence this crate has.

use stasis_detect::{
    find, window, DetectedGame, DpiAwareness, GameTarget, GraphicsApi, Lifecycle, LifecycleAction,
    LifecycleEvent, LifecycleState, PresentationMode, RendererSupport,
};

/// The class the harness registers. It doubles as the marker the detector
/// matches on, which is the reason the harness registers a name of its own.
const HARNESS_CLASS: &str = "StasisHarnessWindow";
const HARNESS_EXECUTABLE: &str = "stasis-harness.exe";

/// The harness, or `None` with a note on stdout saying how to make the test
/// mean something.
fn harness() -> Option<DetectedGame> {
    // Per-monitor awareness has to be in force before any rectangle is read, or
    // Windows hands back coordinates it has scaled behind the caller's back.
    let awareness = stasis_detect::adopt_per_monitor_dpi_awareness();
    assert_eq!(
        awareness,
        DpiAwareness::PerMonitorV2,
        "the test process could not become per monitor DPI aware, so no \
         rectangle below can be trusted"
    );

    let found = find(&GameTarget::WindowClass(HARNESS_CLASS.into()));
    if found.is_none() {
        println!(
            "skipped: no {HARNESS_CLASS} window. Start the harness with \
             `cargo run -p stasis-harness` from engine/ and run this again."
        );
    }
    found
}

#[test]
fn the_running_harness_is_found_and_described() {
    let Some(game) = harness() else { return };

    println!("pid          : {}", game.pid);
    println!("executable   : {:?}", game.executable);
    println!("hwnd         : 0x{:x}", game.window.hwnd());
    println!("class        : {}", game.window.class_name());
    println!("title        : {}", game.window.title());
    println!("client size  : {:?}", game.window.size());
    println!("bounds       : {:?}", game.window.facts.bounds);
    println!("presentation : {:?}", game.window.presentation);
    println!("graphics     : {:?}", game.graphics.loaded());
    println!("support      : {}", game.support());
    if let Some(monitor) = game.window.monitor() {
        println!(
            "monitor      : {} {:?} at {} dpi, scale {}, primary {}",
            monitor.device_name,
            monitor.bounds,
            monitor.dpi,
            monitor.scale(),
            monitor.primary
        );
    }

    assert_ne!(game.pid, 0);
    assert_eq!(game.window.class_name(), HARNESS_CLASS);
    assert_eq!(
        game.window.title(),
        "Stasis harness (pretend this is a game)"
    );
    assert_eq!(game.executable.as_deref(), Some(HARNESS_EXECUTABLE));
}

#[test]
fn the_harness_is_identified_as_direct3d_11_and_therefore_supported() {
    let Some(game) = harness() else { return };

    assert_eq!(
        game.graphics.primary(),
        Some(GraphicsApi::Direct3D11),
        "loaded graphics runtimes were {:?}",
        game.graphics.loaded()
    );
    assert_eq!(
        game.support(),
        RendererSupport::Supported(GraphicsApi::Direct3D11)
    );
}

#[test]
fn the_harness_is_windowed_rather_than_fullscreen() {
    let Some(game) = harness() else { return };

    assert_eq!(
        game.window.presentation,
        Some(PresentationMode::Windowed),
        "the harness uses WS_OVERLAPPEDWINDOW, so anything else is a bug here"
    );
    assert_eq!(game.window.allows_composited_overlay(), Some(true));
}

#[test]
fn the_harness_is_on_a_monitor_whose_dpi_can_be_read() {
    let Some(game) = harness() else { return };
    let monitor = game
        .window
        .monitor()
        .expect("a monitor for a visible window");

    // 96 is the floor Windows reports, and 480 is 500 percent scaling, which is
    // the top of the settings slider. A value outside that is a misread rather
    // than an unusual display.
    assert!(
        (96..=480).contains(&monitor.dpi),
        "implausible DPI {}",
        monitor.dpi
    );
    assert!(!monitor.bounds.is_empty());

    let bounds = game.window.facts.bounds;
    let screen = monitor.bounds;
    assert!(
        bounds.left < screen.right && bounds.right > screen.left,
        "the window at {bounds:?} does not overlap its own monitor at {screen:?}"
    );
}

#[test]
fn the_harness_is_found_by_executable_name_as_well_as_by_window_class() {
    let Some(by_class) = harness() else { return };

    let by_name = find(&GameTarget::Executable(HARNESS_EXECUTABLE.into()))
        .expect("the same process, found the other way");

    assert_eq!(by_name.window.hwnd(), by_class.window.hwnd());
    assert_eq!(by_name.pid, by_class.pid);
}

#[test]
fn a_known_pid_reaches_the_same_window_without_any_process_snapshot() {
    let Some(game) = harness() else { return };

    let by_pid = find(&GameTarget::Pid(game.pid)).expect("the same process, by id");
    assert_eq!(by_pid.window.hwnd(), game.window.hwnd());
}

#[test]
fn every_other_window_the_harness_owns_is_rejected_with_a_reason() {
    let Some(game) = harness() else { return };

    let owned = stasis_detect::windows_of(game.pid);
    println!("the harness process owns {} top level windows", owned.len());

    for facts in &owned {
        match window::rejection(facts) {
            Some(reason) => println!(
                "  rejected 0x{:x} ({}): {reason}",
                facts.hwnd, facts.class_name
            ),
            None => println!("  accepted 0x{:x} ({})", facts.hwnd, facts.class_name),
        }
    }

    let accepted: Vec<isize> = owned
        .iter()
        .filter(|facts| window::rejection(facts).is_none())
        .map(|facts| facts.hwnd)
        .collect();

    assert!(
        accepted.contains(&game.window.hwnd()),
        "the chosen window is not one of the accepted ones"
    );
}

#[test]
fn the_lifecycle_runs_from_finding_the_harness_to_asking_for_an_attach() {
    let Some(game) = harness() else { return };

    let mut lifecycle = Lifecycle::new();
    assert_eq!(
        lifecycle.handle(LifecycleEvent::ProcessStarted),
        LifecycleAction::Nothing
    );

    // The renderer is checked before the window, because an unsupported one has
    // to stop the attach rather than be discovered after it.
    let action = match game.support() {
        RendererSupport::Supported(_) => lifecycle.handle(LifecycleEvent::RenderWindowFound),
        RendererSupport::Unsupported(api) => {
            lifecycle.handle(LifecycleEvent::RendererUnsupported(api))
        }
        other => panic!("the harness should have a readable D3D11 module list, got {other}"),
    };

    assert_eq!(action, LifecycleAction::Attach);
    assert_eq!(lifecycle.state(), LifecycleState::Attaching);
}

#[test]
fn a_game_that_is_not_running_is_reported_as_absent() {
    // Runs whether or not the harness is up, because it is the case that keeps
    // the whole suite honest: nothing found has to mean nothing found.
    assert_eq!(
        find(&GameTarget::WindowClass(
            "StasisClassThatNobodyRegisters".into()
        )),
        None
    );
    assert_eq!(
        find(&GameTarget::Executable("stasis-no-such-game.exe".into())),
        None
    );
    // Process ids are multiples of four, so this one is never assigned.
    assert_eq!(find(&GameTarget::Pid(u32::MAX - 1)), None);
}
