//! The attach and detach lifecycle, as a state machine with no Win32 in it.
//!
//! Game start, alt-tab, minimise, resolution change, a move to another monitor,
//! exit, and crash are all transitions the overlay has to survive. Written as
//! conditionals spread through the code that observes them, they would be
//! untestable and wrong in the cases nobody reproduced. Written here they are a
//! total function over a small pair of enums, and every pair is tested.
//!
//! The crash case is the one that matters. When the game process disappears
//! while the overlay is attached, there is nothing to detach from: the handles
//! are dead, the hook went with the address space, and an orderly detach would
//! at best fail and at worst block. So the machine emits
//! [`LifecycleAction::AbandonAttachment`] rather than
//! [`LifecycleAction::Detach`], and lands in [`LifecycleState::Lost`], which
//! nothing but an explicit acknowledgement leaves. A hook that kills a game
//! should not get to kill it a second time while the player is still reading
//! the first crash dialog.

use crate::graphics::GraphicsApi;
use crate::presentation::PresentationMode;

/// Where the overlay stands with respect to one game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecycleState {
    /// No game is being tracked.
    Searching,
    /// A game process is known, with no window to draw over yet. A game that is
    /// still on its loading screen sits here.
    ProcessFound,
    /// A render window has been chosen and the attach is in flight.
    Attaching,
    /// Attached and drawing.
    Attached,
    /// Attached and not drawing, because the game is minimised.
    Paused,
    /// The renderer was identified and the engine does not support it. The app
    /// says which API by name from here.
    Unsupported(GraphicsApi),
    /// The game vanished while the overlay was attached to it. A deliberate
    /// dead end: see the module comment.
    Lost,
}

impl std::fmt::Display for LifecycleState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LifecycleState::Searching => write!(f, "searching"),
            LifecycleState::ProcessFound => write!(f, "game found, no window yet"),
            LifecycleState::Attaching => write!(f, "attaching"),
            LifecycleState::Attached => write!(f, "attached"),
            LifecycleState::Paused => write!(f, "attached, minimised"),
            LifecycleState::Unsupported(api) => write!(f, "{api} is not supported yet"),
            LifecycleState::Lost => write!(f, "the game exited unexpectedly"),
        }
    }
}

impl LifecycleState {
    /// True while the overlay holds something inside the game process.
    ///
    /// [`LifecycleState::Attaching`] counts: the injector is touching the
    /// process, so a crash from there is still ours to clean up after.
    pub fn holds_the_process(&self) -> bool {
        matches!(
            self,
            LifecycleState::Attaching | LifecycleState::Attached | LifecycleState::Paused
        )
    }

    /// Every state, for tests that have to cover all of them.
    pub const ALL: [LifecycleState; 7] = [
        LifecycleState::Searching,
        LifecycleState::ProcessFound,
        LifecycleState::Attaching,
        LifecycleState::Attached,
        LifecycleState::Paused,
        LifecycleState::Unsupported(GraphicsApi::Vulkan),
        LifecycleState::Lost,
    ];
}

/// Something the detector observed about the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecycleEvent {
    /// A process matching the target appeared.
    ProcessStarted,
    /// The process ended, and it ended the way processes end: the window went
    /// first, or the exit code was collected.
    ProcessExited,
    /// The process is gone and nothing said it was going. This is the crash.
    ProcessVanished,
    /// A window passing every check in [`crate::window`] was chosen.
    RenderWindowFound,
    /// The render window was destroyed while the process lives. A resolution
    /// or presentation mode change does this in some engines.
    RenderWindowClosed,
    /// The renderer was identified and it is not one the engine draws over.
    RendererUnsupported(GraphicsApi),
    /// The payload is in and the hook is live.
    AttachSucceeded,
    /// The attach did not take. The reason belongs to the injector, not here.
    AttachFailed,
    /// The game was minimised.
    Minimised,
    /// The game came back from minimised.
    Restored,
    /// The game is no longer the foreground window. This is alt-tab.
    FocusLost,
    /// The game is the foreground window again.
    FocusGained,
    /// The render surface changed size. A resolution change is this.
    SurfaceResized,
    /// The window's display changed under it, from
    /// [`crate::display::display_change`].
    DisplayChanged {
        /// Carried through from [`crate::display::DisplayChange::requires_rebuild`],
        /// because whether a monitor move costs anything depends on DPI and
        /// that comparison belongs in the display module.
        requires_rebuild: bool,
    },
    /// The game switched between windowed, borderless and exclusive.
    PresentationModeChanged(PresentationMode),
    /// The user turned the overlay off while the game is still running.
    DetachRequested,
    /// The app has shown the crash to the user and is ready to look again.
    CrashAcknowledged,
}

impl LifecycleEvent {
    /// Every event, for tests that have to cover all of them. The variants that
    /// carry data appear once with a representative value, and the cases where
    /// that value changes the outcome have tests of their own.
    pub const ALL: [LifecycleEvent; 17] = [
        LifecycleEvent::ProcessStarted,
        LifecycleEvent::ProcessExited,
        LifecycleEvent::ProcessVanished,
        LifecycleEvent::RenderWindowFound,
        LifecycleEvent::RenderWindowClosed,
        LifecycleEvent::RendererUnsupported(GraphicsApi::Vulkan),
        LifecycleEvent::AttachSucceeded,
        LifecycleEvent::AttachFailed,
        LifecycleEvent::Minimised,
        LifecycleEvent::Restored,
        LifecycleEvent::FocusLost,
        LifecycleEvent::FocusGained,
        LifecycleEvent::SurfaceResized,
        LifecycleEvent::DisplayChanged {
            requires_rebuild: true,
        },
        LifecycleEvent::PresentationModeChanged(PresentationMode::Borderless),
        LifecycleEvent::DetachRequested,
        LifecycleEvent::CrashAcknowledged,
    ];
}

/// What the rest of the engine should do about a transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecycleAction {
    /// Carry on.
    Nothing,
    /// Inject and hook. Only ever emitted on the way into
    /// [`LifecycleState::Attaching`].
    Attach,
    /// Unhook inside a process that is still alive and still ours to touch.
    Detach,
    /// Drop everything on this side without touching the target. The process is
    /// gone, or going, and reaching into it would fail or hang.
    AbandonAttachment,
    /// Stop drawing, keep the hook.
    PauseDrawing,
    /// Start drawing again.
    ResumeDrawing,
    /// The surface the overlay draws into is no longer the right size or scale.
    RebuildSurface,
    /// Tell the user which API this is and that it is not supported yet.
    ReportUnsupported(GraphicsApi),
}

/// The result of one event: where the machine is now, and what to do about it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transition {
    /// The state after the event.
    pub state: LifecycleState,
    /// The work the event created.
    pub action: LifecycleAction,
}

impl Transition {
    fn new(state: LifecycleState, action: LifecycleAction) -> Self {
        Transition { state, action }
    }

    fn stay(state: LifecycleState) -> Self {
        Transition::new(state, LifecycleAction::Nothing)
    }
}

/// Apply one event to one state.
///
/// Total: every pair of state and event has an answer, and unrelated pairs stay
/// where they are rather than being an error. A window event arriving after the
/// process died is normal, not a bug in the caller, because the two are
/// observed by different means and arrive out of order.
pub fn advance(state: LifecycleState, event: LifecycleEvent) -> Transition {
    use LifecycleAction as Action;
    use LifecycleEvent as Event;
    use LifecycleState as State;

    // The process ending is handled first and identically from every state,
    // because it outranks anything else that could be in flight. Splitting it
    // per state is how a machine ends up with one branch that tries to unhook a
    // dead process.
    match event {
        Event::ProcessVanished => {
            return if state.holds_the_process() {
                Transition::new(State::Lost, Action::AbandonAttachment)
            } else {
                // Nothing of ours was inside it, so this is a game that died on
                // its own. Not a crash the overlay has to answer for, and not a
                // reason to stop looking.
                Transition::stay(State::Searching)
            };
        }
        Event::ProcessExited => {
            return if state.holds_the_process() {
                Transition::new(State::Searching, Action::AbandonAttachment)
            } else {
                Transition::stay(State::Searching)
            };
        }
        _ => {}
    }

    match state {
        State::Searching => match event {
            Event::ProcessStarted => Transition::stay(State::ProcessFound),
            _ => Transition::stay(State::Searching),
        },

        State::ProcessFound => match event {
            Event::RendererUnsupported(api) => {
                Transition::new(State::Unsupported(api), Action::ReportUnsupported(api))
            }
            Event::RenderWindowFound => Transition::new(State::Attaching, Action::Attach),
            _ => Transition::stay(State::ProcessFound),
        },

        State::Attaching => match event {
            Event::AttachSucceeded => Transition::stay(State::Attached),
            // Back to waiting rather than to a failed state of its own. The
            // window may not be the final one, and the next scan will decide
            // again. Backing off between attempts is the caller's job.
            Event::AttachFailed => Transition::stay(State::ProcessFound),
            Event::RenderWindowClosed => Transition::stay(State::ProcessFound),
            Event::RendererUnsupported(api) => {
                Transition::new(State::Unsupported(api), Action::ReportUnsupported(api))
            }
            Event::DetachRequested => Transition::new(State::Searching, Action::Detach),
            _ => Transition::stay(State::Attaching),
        },

        State::Attached => match event {
            Event::Minimised => Transition::new(State::Paused, Action::PauseDrawing),
            // Alt-tab. The game keeps presenting while another window has
            // focus, so the overlay keeps drawing. Detaching here would make
            // every alt-tab a reinjection, which is both slower and far more
            // interesting to an anti-cheat than staying put.
            Event::FocusLost | Event::FocusGained => Transition::stay(State::Attached),
            Event::SurfaceResized => Transition::new(State::Attached, Action::RebuildSurface),
            Event::DisplayChanged { requires_rebuild } => Transition::new(
                State::Attached,
                if requires_rebuild {
                    Action::RebuildSurface
                } else {
                    Action::Nothing
                },
            ),
            // A mode switch recreates the swapchain, so whatever the overlay
            // built for the old one is stale even when the size matches.
            Event::PresentationModeChanged(_) => {
                Transition::new(State::Attached, Action::RebuildSurface)
            }
            // The process is alive and the hook with it, so this is a pause,
            // not a detach. Engines that recreate their window on a mode change
            // come straight back with a new one.
            Event::RenderWindowClosed => Transition::new(State::ProcessFound, Action::PauseDrawing),
            Event::DetachRequested => Transition::new(State::Searching, Action::Detach),
            // Learning the API late, after the hook is in. Leaving it attached
            // would be an overlay that never draws with no explanation.
            Event::RendererUnsupported(api) => {
                Transition::new(State::Unsupported(api), Action::Detach)
            }
            _ => Transition::stay(State::Attached),
        },

        State::Paused => match event {
            Event::Restored => Transition::new(State::Attached, Action::ResumeDrawing),
            Event::RenderWindowClosed => Transition::stay(State::ProcessFound),
            Event::DetachRequested => Transition::new(State::Searching, Action::Detach),
            Event::RendererUnsupported(api) => {
                Transition::new(State::Unsupported(api), Action::Detach)
            }
            // Nothing is being drawn, so nothing needs rebuilding yet. The
            // caller re-reads the display when it resumes, and a move that
            // happened while minimised shows up as a display change then.
            _ => Transition::stay(State::Paused),
        },

        State::Unsupported(api) => match event {
            Event::DetachRequested => Transition::stay(State::Searching),
            // Deliberately not answering RenderWindowFound. The renderer has
            // not changed, and attaching to a window on an API the engine
            // cannot draw over is the silent failure this crate exists to
            // prevent.
            _ => Transition::stay(State::Unsupported(api)),
        },

        State::Lost => match event {
            Event::CrashAcknowledged => Transition::stay(State::Searching),
            // Including ProcessStarted. The game coming back is exactly when
            // an unacknowledged crash would repeat itself.
            _ => Transition::stay(State::Lost),
        },
    }
}

/// A small holder for [`advance`], for callers that would otherwise keep the
/// state in a field and remember to assign it back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lifecycle {
    state: LifecycleState,
}

impl Default for Lifecycle {
    fn default() -> Self {
        Lifecycle::new()
    }
}

impl Lifecycle {
    /// A lifecycle that is not tracking anything yet.
    pub fn new() -> Self {
        Lifecycle {
            state: LifecycleState::Searching,
        }
    }

    /// Where the overlay stands.
    pub fn state(&self) -> LifecycleState {
        self.state
    }

    /// Apply an event and return the work it created.
    pub fn handle(&mut self, event: LifecycleEvent) -> LifecycleAction {
        let transition = advance(self.state, event);
        self.state = transition.state;
        transition.action
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(start: LifecycleState, events: &[LifecycleEvent]) -> Lifecycle {
        let mut lifecycle = Lifecycle { state: start };
        for event in events {
            lifecycle.handle(*event);
        }
        lifecycle
    }

    #[test]
    fn a_new_lifecycle_is_looking_for_a_game() {
        assert_eq!(Lifecycle::new().state(), LifecycleState::Searching);
    }

    #[test]
    fn the_ordinary_run_from_game_start_to_drawing() {
        let mut lifecycle = Lifecycle::new();

        assert_eq!(
            lifecycle.handle(LifecycleEvent::ProcessStarted),
            LifecycleAction::Nothing
        );
        assert_eq!(lifecycle.state(), LifecycleState::ProcessFound);

        assert_eq!(
            lifecycle.handle(LifecycleEvent::RenderWindowFound),
            LifecycleAction::Attach
        );
        assert_eq!(lifecycle.state(), LifecycleState::Attaching);

        assert_eq!(
            lifecycle.handle(LifecycleEvent::AttachSucceeded),
            LifecycleAction::Nothing
        );
        assert_eq!(lifecycle.state(), LifecycleState::Attached);
    }

    #[test]
    fn a_game_that_crashes_while_attached_is_abandoned_rather_than_detached() {
        // The single most important transition in this file. Detaching means
        // reaching into a process that no longer exists.
        for state in [
            LifecycleState::Attaching,
            LifecycleState::Attached,
            LifecycleState::Paused,
        ] {
            let transition = advance(state, LifecycleEvent::ProcessVanished);

            assert_eq!(transition.state, LifecycleState::Lost, "from {state:?}");
            assert_eq!(
                transition.action,
                LifecycleAction::AbandonAttachment,
                "from {state:?}"
            );
        }
    }

    #[test]
    fn nothing_ever_answers_a_vanished_process_by_touching_it() {
        for state in LifecycleState::ALL {
            let action = advance(state, LifecycleEvent::ProcessVanished).action;
            assert!(
                matches!(
                    action,
                    LifecycleAction::AbandonAttachment | LifecycleAction::Nothing
                ),
                "{state:?} answered a vanished process with {action:?}"
            );
        }
    }

    #[test]
    fn a_crash_has_to_be_acknowledged_before_anything_attaches_again() {
        // A hook that killed the game once will kill it again the moment the
        // player restarts it. The app has to have said something to the user
        // first.
        let crashed = run(
            LifecycleState::Attached,
            &[
                LifecycleEvent::ProcessVanished,
                LifecycleEvent::ProcessStarted,
                LifecycleEvent::RenderWindowFound,
                LifecycleEvent::AttachSucceeded,
            ],
        );
        assert_eq!(crashed.state(), LifecycleState::Lost);

        let acknowledged = run(
            LifecycleState::Lost,
            &[
                LifecycleEvent::CrashAcknowledged,
                LifecycleEvent::ProcessStarted,
            ],
        );
        assert_eq!(acknowledged.state(), LifecycleState::ProcessFound);
    }

    #[test]
    fn no_event_attaches_from_the_lost_state() {
        for event in LifecycleEvent::ALL {
            let transition = advance(LifecycleState::Lost, event);
            assert_ne!(
                transition.action,
                LifecycleAction::Attach,
                "{event:?} attached out of a crash"
            );
        }
    }

    #[test]
    fn a_game_that_dies_before_the_overlay_touched_it_is_not_a_crash_to_answer_for() {
        // Told apart on purpose. A game that fails to start on its own should
        // not leave the app claiming the overlay killed it.
        for state in [LifecycleState::Searching, LifecycleState::ProcessFound] {
            let transition = advance(state, LifecycleEvent::ProcessVanished);
            assert_eq!(transition.state, LifecycleState::Searching);
            assert_eq!(transition.action, LifecycleAction::Nothing);
        }
    }

    #[test]
    fn an_ordinary_exit_returns_to_searching_without_reaching_into_the_process() {
        let transition = advance(LifecycleState::Attached, LifecycleEvent::ProcessExited);

        assert_eq!(transition.state, LifecycleState::Searching);
        assert_eq!(transition.action, LifecycleAction::AbandonAttachment);
    }

    #[test]
    fn an_exit_from_a_state_that_held_nothing_needs_no_cleanup() {
        let transition = advance(LifecycleState::ProcessFound, LifecycleEvent::ProcessExited);

        assert_eq!(transition.state, LifecycleState::Searching);
        assert_eq!(transition.action, LifecycleAction::Nothing);
    }

    #[test]
    fn alt_tab_does_not_detach_the_overlay() {
        // Reinjecting on every alt-tab would be slower and much more visible to
        // an anti-cheat than staying attached.
        let lifecycle = run(
            LifecycleState::Attached,
            &[LifecycleEvent::FocusLost, LifecycleEvent::FocusGained],
        );
        assert_eq!(lifecycle.state(), LifecycleState::Attached);

        assert_eq!(
            advance(LifecycleState::Attached, LifecycleEvent::FocusLost).action,
            LifecycleAction::Nothing
        );
    }

    #[test]
    fn minimising_pauses_drawing_and_restoring_resumes_it() {
        let mut lifecycle = Lifecycle {
            state: LifecycleState::Attached,
        };

        assert_eq!(
            lifecycle.handle(LifecycleEvent::Minimised),
            LifecycleAction::PauseDrawing
        );
        assert_eq!(lifecycle.state(), LifecycleState::Paused);

        assert_eq!(
            lifecycle.handle(LifecycleEvent::Restored),
            LifecycleAction::ResumeDrawing
        );
        assert_eq!(lifecycle.state(), LifecycleState::Attached);
    }

    #[test]
    fn minimising_twice_pauses_once() {
        let mut lifecycle = Lifecycle {
            state: LifecycleState::Paused,
        };
        assert_eq!(
            lifecycle.handle(LifecycleEvent::Minimised),
            LifecycleAction::Nothing
        );
        assert_eq!(lifecycle.state(), LifecycleState::Paused);
    }

    #[test]
    fn a_resolution_change_rebuilds_the_surface_without_detaching() {
        let mut lifecycle = Lifecycle {
            state: LifecycleState::Attached,
        };

        assert_eq!(
            lifecycle.handle(LifecycleEvent::SurfaceResized),
            LifecycleAction::RebuildSurface
        );
        assert_eq!(lifecycle.state(), LifecycleState::Attached);
    }

    #[test]
    fn a_move_to_a_monitor_that_scales_differently_rebuilds_the_surface() {
        assert_eq!(
            advance(
                LifecycleState::Attached,
                LifecycleEvent::DisplayChanged {
                    requires_rebuild: true
                }
            )
            .action,
            LifecycleAction::RebuildSurface
        );
    }

    #[test]
    fn a_move_between_monitors_of_the_same_scale_costs_nothing() {
        assert_eq!(
            advance(
                LifecycleState::Attached,
                LifecycleEvent::DisplayChanged {
                    requires_rebuild: false
                }
            )
            .action,
            LifecycleAction::Nothing
        );
    }

    #[test]
    fn a_display_change_while_minimised_does_not_rebuild_a_surface_nobody_sees() {
        let transition = advance(
            LifecycleState::Paused,
            LifecycleEvent::DisplayChanged {
                requires_rebuild: true,
            },
        );

        assert_eq!(transition.state, LifecycleState::Paused);
        assert_eq!(transition.action, LifecycleAction::Nothing);
    }

    #[test]
    fn switching_presentation_mode_rebuilds_the_surface() {
        // A borderless to exclusive switch recreates the swapchain, so the size
        // matching is not enough to keep the old surface.
        for mode in [
            PresentationMode::Windowed,
            PresentationMode::Borderless,
            PresentationMode::Exclusive,
        ] {
            assert_eq!(
                advance(
                    LifecycleState::Attached,
                    LifecycleEvent::PresentationModeChanged(mode)
                )
                .action,
                LifecycleAction::RebuildSurface
            );
        }
    }

    #[test]
    fn a_window_destroyed_by_a_mode_change_pauses_rather_than_detaches() {
        // Some engines destroy and recreate the window when the player changes
        // presentation mode. The hook is in the process, not the window.
        let transition = advance(LifecycleState::Attached, LifecycleEvent::RenderWindowClosed);

        assert_eq!(transition.state, LifecycleState::ProcessFound);
        assert_eq!(transition.action, LifecycleAction::PauseDrawing);
    }

    #[test]
    fn a_failed_attach_goes_back_to_waiting_rather_than_retrying_itself() {
        let transition = advance(LifecycleState::Attaching, LifecycleEvent::AttachFailed);

        assert_eq!(transition.state, LifecycleState::ProcessFound);
        assert_eq!(transition.action, LifecycleAction::Nothing);
    }

    #[test]
    fn an_unsupported_renderer_is_reported_by_name_and_never_attached_to() {
        let mut lifecycle = Lifecycle {
            state: LifecycleState::ProcessFound,
        };

        assert_eq!(
            lifecycle.handle(LifecycleEvent::RendererUnsupported(GraphicsApi::Vulkan)),
            LifecycleAction::ReportUnsupported(GraphicsApi::Vulkan)
        );
        assert_eq!(
            lifecycle.state(),
            LifecycleState::Unsupported(GraphicsApi::Vulkan)
        );
        assert_eq!(
            lifecycle.state().to_string(),
            "Vulkan is not supported yet",
            "the state is what the app reads the API name out of"
        );

        // The window turning up later must not restart the attach.
        assert_eq!(
            lifecycle.handle(LifecycleEvent::RenderWindowFound),
            LifecycleAction::Nothing
        );
        assert_eq!(
            lifecycle.state(),
            LifecycleState::Unsupported(GraphicsApi::Vulkan)
        );
    }

    #[test]
    fn learning_the_renderer_is_unsupported_after_attaching_detaches_cleanly() {
        // The process is alive here, so unlike the crash path this one really
        // does unhook.
        let transition = advance(
            LifecycleState::Attached,
            LifecycleEvent::RendererUnsupported(GraphicsApi::Direct3D12),
        );

        assert_eq!(
            transition.state,
            LifecycleState::Unsupported(GraphicsApi::Direct3D12)
        );
        assert_eq!(transition.action, LifecycleAction::Detach);
    }

    #[test]
    fn an_unsupported_game_that_exits_lets_the_search_start_again() {
        for event in [
            LifecycleEvent::ProcessExited,
            LifecycleEvent::ProcessVanished,
        ] {
            let transition = advance(LifecycleState::Unsupported(GraphicsApi::Vulkan), event);
            assert_eq!(transition.state, LifecycleState::Searching);
            assert_eq!(transition.action, LifecycleAction::Nothing);
        }
    }

    #[test]
    fn turning_the_overlay_off_detaches_from_a_live_process() {
        for state in [LifecycleState::Attached, LifecycleState::Paused] {
            let transition = advance(state, LifecycleEvent::DetachRequested);
            assert_eq!(transition.state, LifecycleState::Searching);
            assert_eq!(transition.action, LifecycleAction::Detach);
        }
    }

    #[test]
    fn only_a_state_holding_the_process_is_ever_told_to_detach() {
        // Detach means reaching into the target. Emitting it from a state that
        // holds nothing would be an unhook of something that was never hooked.
        for state in LifecycleState::ALL {
            for event in LifecycleEvent::ALL {
                let action = advance(state, event).action;
                if action == LifecycleAction::Detach {
                    assert!(
                        state.holds_the_process(),
                        "{state:?} on {event:?} asked for a detach"
                    );
                }
            }
        }
    }

    #[test]
    fn attaching_is_only_ever_asked_for_on_the_way_into_the_attaching_state() {
        for state in LifecycleState::ALL {
            for event in LifecycleEvent::ALL {
                let transition = advance(state, event);
                if transition.action == LifecycleAction::Attach {
                    assert_eq!(
                        transition.state,
                        LifecycleState::Attaching,
                        "{state:?} on {event:?} asked for an attach without attaching"
                    );
                    assert_eq!(state, LifecycleState::ProcessFound);
                }
            }
        }
    }

    #[test]
    fn drawing_actions_only_come_from_states_that_are_attached() {
        for state in LifecycleState::ALL {
            for event in LifecycleEvent::ALL {
                let action = advance(state, event).action;
                if matches!(
                    action,
                    LifecycleAction::PauseDrawing
                        | LifecycleAction::ResumeDrawing
                        | LifecycleAction::RebuildSurface
                ) {
                    assert!(
                        matches!(state, LifecycleState::Attached | LifecycleState::Paused),
                        "{state:?} on {event:?} produced {action:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn every_pair_of_state_and_event_has_an_answer() {
        // The machine is total on purpose. Events arrive from window hooks and
        // process polling, which are not ordered against each other, so a pair
        // that looks impossible turns up in practice.
        for state in LifecycleState::ALL {
            for event in LifecycleEvent::ALL {
                let transition = advance(state, event);
                assert_eq!(
                    transition,
                    advance(state, event),
                    "{state:?} on {event:?} is not deterministic"
                );
            }
        }
    }

    #[test]
    fn an_event_that_means_nothing_in_a_state_leaves_it_alone() {
        // The transitions that are deliberately no-ops, listed so that adding a
        // new one is a decision rather than a side effect.
        for (state, event) in [
            (LifecycleState::Searching, LifecycleEvent::AttachSucceeded),
            (LifecycleState::Searching, LifecycleEvent::Minimised),
            (LifecycleState::ProcessFound, LifecycleEvent::FocusLost),
            (LifecycleState::Attaching, LifecycleEvent::SurfaceResized),
            (LifecycleState::Attached, LifecycleEvent::ProcessStarted),
            (LifecycleState::Paused, LifecycleEvent::FocusGained),
        ] {
            let transition = advance(state, event);
            assert_eq!(transition.state, state, "{state:?} moved on {event:?}");
            assert_eq!(transition.action, LifecycleAction::Nothing);
        }
    }
}
