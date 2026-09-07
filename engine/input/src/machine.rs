//! The input state machine, with no clock, no Windows, and no I/O in it.
//!
//! Everything here is a pure function of the current state, one event, and a
//! millisecond reading the caller supplies. That is deliberate: the property
//! this crate exists to guarantee is a property about every reachable state and
//! every ordering of events, and the only way to check that is to be able to
//! enumerate it in a test rather than reproduce it against a running game.

use crate::Millis;

/// How long an acquire may stay in flight before it is abandoned.
///
/// Acquiring means the filter is already swallowing input while the overlay is
/// brought to the front. If the confirmation has not arrived by then, the
/// foreground change is not going to land, and the half-acquired state is
/// exactly the one that eats the player's input.
pub const ACQUIRE_DEADLINE_MS: Millis = 500;

/// How long a release may stay in flight before it is treated as failed.
///
/// The release runs through the game's own message pump, so it is bounded by
/// the game's frame time rather than ours. At 20 fps this is still fifteen
/// frames of slack, and it is short enough that a player who has just pressed
/// the toggle has not yet decided the game is broken.
pub const RELEASE_DEADLINE_MS: Millis = 750;

/// How long a published [`Capture`] stays valid without being refreshed.
///
/// This is the number that makes the guarantee hold when the supervisor is the
/// thing that broke. See the crate documentation for why the decision is leased
/// rather than latched.
pub const CAPTURE_LEASE_MS: Millis = 1_000;

/// Where input is, and what is being done about it.
///
/// Only [`InputState::Released`] means the game has its input. Every other
/// state must be treated as holding it, including the two transient ones,
/// because a half-finished handover is indistinguishable from a finished one
/// from the outside.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputState {
    /// Click-through. The overlay is not in the input path and the game has
    /// every key and click. This is the state the whole crate is built to
    /// return to.
    Released,
    /// The filter is swallowing input and the overlay is being brought to the
    /// front, but nothing has confirmed that it got there.
    Acquiring,
    /// The overlay has focus and input. The only state a user ever means to be
    /// in, and the only holding state without a deadline on it.
    Captured,
    /// Input is being handed back and the handover has not been confirmed.
    Releasing,
    /// A release ran past its deadline. Input has to be assumed captured, and
    /// nothing the overlay says is trusted from here.
    Stuck,
}

impl InputState {
    /// Whether the game is being denied input in this state.
    ///
    /// True for everything except [`InputState::Released`]. The transient
    /// states count as holding on purpose: reporting a maybe as a no would let
    /// a stalled handover pass for a finished one.
    pub fn holds_input(self) -> bool {
        !matches!(self, InputState::Released)
    }

    /// Every state, for tests and for anything that needs to enumerate them.
    pub const ALL: [InputState; 5] = [
        InputState::Released,
        InputState::Acquiring,
        InputState::Captured,
        InputState::Releasing,
        InputState::Stuck,
    ];
}

/// Something that happened, fed to the machine by whoever observed it.
///
/// The set splits in two, and the split is what the release guarantee rests on.
/// [`InputEvent::Tick`] and [`InputEvent::ForcedRelease`] are produced by the
/// supervisor out of its own clock and its own teardown, so they are available
/// even when the overlay is dead. Everything else needs the overlay, the
/// window manager, or the user to cooperate, and therefore cannot be relied on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputEvent {
    /// The bound hotkey fired. The toggle, from either direction.
    HotkeyPressed,
    /// The overlay has focus and the filter is swallowing. Honoured only while
    /// acquiring.
    CaptureConfirmed,
    /// The acquire could not be completed. The window may still be swallowing,
    /// so this owes a release rather than a jump back to released.
    CaptureFailed,
    /// The game has its input back. Honoured only while releasing.
    ReleaseConfirmed,
    /// The overlay stopped being the foreground window: alt-tab, a click into
    /// the game, a UAC prompt, the game going exclusive fullscreen.
    FocusLost,
    /// The overlay stopped answering, whether it exited or hung.
    OverlayLost,
    /// The supervisor tore capture down without the overlay's help. The one
    /// edge that leads to released from anywhere.
    ForcedRelease,
    /// Nothing happened, time passed. Carried as an event so a deadline can
    /// expire without needing something else to expire on.
    Tick,
}

impl InputEvent {
    /// Every event, for tests and for anything that needs to enumerate them.
    pub const ALL: [InputEvent; 8] = [
        InputEvent::HotkeyPressed,
        InputEvent::CaptureConfirmed,
        InputEvent::CaptureFailed,
        InputEvent::ReleaseConfirmed,
        InputEvent::FocusLost,
        InputEvent::OverlayLost,
        InputEvent::ForcedRelease,
        InputEvent::Tick,
    ];

    /// The events the supervisor can raise on its own, with no help from the
    /// overlay, the game, or the user.
    ///
    /// The reachability test is stated over exactly this set. A route back to
    /// released that runs through [`InputEvent::ReleaseConfirmed`] is not a
    /// route, because the party that would send it is the party that is
    /// suspected of being hung.
    pub const UNILATERAL: [InputEvent; 2] = [InputEvent::Tick, InputEvent::ForcedRelease];
}

/// What the caller has to go and do after a transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Nothing to do.
    None,
    /// Start swallowing input and bring the overlay to the front.
    Acquire,
    /// Stop swallowing and hand the game back. Must be idempotent, because a
    /// retry after a partial failure is a normal case.
    Release,
    /// Tear capture down by a route that does not run any overlay code. The
    /// supervisor owes this one and it has to be the last thing that can fail.
    ForceRelease,
}

/// The result of feeding one event to the machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transition {
    /// The state before the event.
    pub from: InputState,
    /// The state after it.
    pub to: InputState,
    /// What the caller has to do about it.
    pub effect: Effect,
}

impl Transition {
    /// Whether the state actually moved, as opposed to the event being ignored.
    pub fn changed(&self) -> bool {
        self.from != self.to
    }
}

/// The decision the input filter reads, and the reading after which it stops
/// being true.
///
/// The expiry is the point of the type. A filter that reads a plain boolean
/// keeps swallowing forever if whoever set it stops running, so the boolean is
/// paired with a deadline that the filter checks against its own clock. The
/// safe state is then the one that needs nobody to do anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capture {
    /// Whether input belonged to the overlay at the moment this was published.
    pub swallow_input: bool,
    /// The reading past which `swallow_input` must be read as false.
    pub expires_at: Millis,
}

impl Capture {
    /// A capture that gives the game its input and cannot be revived by a stale
    /// clock reading.
    pub fn released() -> Self {
        Capture {
            swallow_input: false,
            expires_at: 0,
        }
    }

    /// Whether input should be swallowed right now.
    ///
    /// This is the only function the filter in the game process should call,
    /// and it must be called with the filter's own clock rather than the one
    /// that produced the capture.
    pub fn swallows(&self, now: Millis) -> bool {
        self.swallow_input && now < self.expires_at
    }
}

/// The next state, as a function of nothing but its three arguments.
///
/// The rules, in the order they resolve:
///
/// - [`InputEvent::ForcedRelease`] always lands on released. It is the edge
///   that keeps the guarantee true, so it has no exceptions.
/// - Confirmations are good news and are honoured only for the phase actually
///   in flight. A confirmation arriving in any other state is a message from a
///   cycle that has already been abandoned and is ignored.
/// - Bad news, meaning a failed acquire, lost focus, or a lost overlay, drives
///   release from every state that is holding input.
/// - The hotkey toggles: it starts an acquire from released and starts a
///   release from everywhere else, so pressing it twice quickly cannot land in
///   a holding state.
/// - Deadlines are checked only on a tick, and only in the two transient
///   states. Captured has no deadline because a player may legitimately keep
///   the overlay open, and a timer that closes it under them would be a second
///   bug rather than a safeguard.
fn next(state: InputState, event: InputEvent, elapsed: Millis) -> InputState {
    use InputEvent as E;
    use InputState as S;

    if matches!(event, E::ForcedRelease) {
        return S::Released;
    }

    match state {
        S::Released => match event {
            E::HotkeyPressed => S::Acquiring,
            _ => S::Released,
        },
        S::Acquiring => match event {
            E::CaptureConfirmed => S::Captured,
            E::HotkeyPressed | E::CaptureFailed | E::FocusLost | E::OverlayLost => S::Releasing,
            E::Tick if elapsed >= ACQUIRE_DEADLINE_MS => S::Releasing,
            _ => S::Acquiring,
        },
        S::Captured => match event {
            E::HotkeyPressed | E::CaptureFailed | E::FocusLost | E::OverlayLost => S::Releasing,
            _ => S::Captured,
        },
        S::Releasing => match event {
            E::ReleaseConfirmed => S::Released,
            E::Tick if elapsed >= RELEASE_DEADLINE_MS => S::Stuck,
            _ => S::Releasing,
        },
        // Once a release has missed its deadline, nothing the overlay reports
        // is believed again. Only the supervisor's teardown, handled above,
        // gets out of here.
        S::Stuck => S::Stuck,
    }
}

/// What the caller owes for a given move.
///
/// Derived from the arguments rather than stored, so a state and its effect
/// cannot drift apart.
///
/// Being stuck re-asks for the teardown, because a supervisor whose first
/// attempt failed needs another one and nothing else is going to arrive. It
/// re-asks on the tick only, not on every event. That distinction was found by
/// running it: a supervisor that answers each `ForceRelease` by scheduling work
/// a moment later gets a fresh request on every stray event, each one pushing
/// the scheduled work back, and the teardown never actually runs. Tying the
/// retry to the tick puts the retry rate under the caller's own cadence.
fn effect_of(from: InputState, to: InputState, event: InputEvent) -> Effect {
    match (from, to) {
        (f, InputState::Stuck) if f != InputState::Stuck => Effect::ForceRelease,
        (InputState::Stuck, InputState::Stuck) if matches!(event, InputEvent::Tick) => {
            Effect::ForceRelease
        }
        (f, InputState::Acquiring) if f != InputState::Acquiring => Effect::Acquire,
        (f, InputState::Releasing) if f != InputState::Releasing => Effect::Release,
        _ => Effect::None,
    }
}

/// The state machine, plus the reading at which the current state was entered.
///
/// Holds no clock. Every method that needs the time takes it, which is what
/// lets the tests walk years of deadlines without sleeping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Machine {
    state: InputState,
    entered_at: Millis,
}

impl Machine {
    /// A machine in the safe state, entered at `now`.
    pub fn new(now: Millis) -> Self {
        Machine {
            state: InputState::Released,
            entered_at: now,
        }
    }

    /// The current state.
    pub fn state(&self) -> InputState {
        self.state
    }

    /// Whether the game is being denied input.
    pub fn holds_input(&self) -> bool {
        self.state.holds_input()
    }

    /// How long the machine has been in its current state.
    ///
    /// Saturating, so a clock that goes backwards reads as zero rather than
    /// wrapping into a deadline that has already expired.
    pub fn elapsed(&self, now: Millis) -> Millis {
        now.saturating_sub(self.entered_at)
    }

    /// Feed one event and get back what to do about it.
    ///
    /// A self-loop deliberately does not refresh `entered_at`. Without that, a
    /// stream of focus changes during a release would push the deadline out
    /// indefinitely and the escalation would never fire, which is the exact
    /// shape of the bug this machine is meant to prevent.
    pub fn on(&mut self, event: InputEvent, now: Millis) -> Transition {
        let from = self.state;
        let to = next(from, event, self.elapsed(now));

        if to != from {
            self.state = to;
            self.entered_at = now;
        }

        Transition {
            from,
            to,
            effect: effect_of(from, to, event),
        }
    }

    /// The decision to publish for the input filter to read.
    ///
    /// Call this every time the supervisor's loop runs, not only when the state
    /// changes. The expiry is what a stalled supervisor stops refreshing, and a
    /// value that is only republished on change never expires while nothing is
    /// happening.
    pub fn capture(&self, now: Millis) -> Capture {
        Capture {
            swallow_input: self.state.holds_input(),
            expires_at: now.saturating_add(CAPTURE_LEASE_MS),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashSet, VecDeque};

    use InputEvent as E;
    use InputState as S;

    /// Long enough to expire either deadline, so a test that wants "the clock
    /// ran out" does not have to know which state it is in.
    const PAST_ANY_DEADLINE: Millis = 10_000;

    fn machine_in(state: S) -> Machine {
        Machine {
            state,
            entered_at: 0,
        }
    }

    // --- the transition table ---------------------------------------------

    /// Every cell of the table, written out rather than derived, so that a
    /// change to `next` has to be argued for here before it lands.
    ///
    /// Read as (state, event, elapsed past the deadline, expected state).
    const TABLE: &[(S, E, bool, S)] = &[
        (S::Released, E::HotkeyPressed, false, S::Acquiring),
        (S::Released, E::CaptureConfirmed, false, S::Released),
        (S::Released, E::CaptureFailed, false, S::Released),
        (S::Released, E::ReleaseConfirmed, false, S::Released),
        (S::Released, E::FocusLost, false, S::Released),
        (S::Released, E::OverlayLost, false, S::Released),
        (S::Released, E::ForcedRelease, false, S::Released),
        (S::Released, E::Tick, false, S::Released),
        (S::Released, E::Tick, true, S::Released),
        (S::Acquiring, E::HotkeyPressed, false, S::Releasing),
        (S::Acquiring, E::CaptureConfirmed, false, S::Captured),
        (S::Acquiring, E::CaptureFailed, false, S::Releasing),
        (S::Acquiring, E::ReleaseConfirmed, false, S::Acquiring),
        (S::Acquiring, E::FocusLost, false, S::Releasing),
        (S::Acquiring, E::OverlayLost, false, S::Releasing),
        (S::Acquiring, E::ForcedRelease, false, S::Released),
        (S::Acquiring, E::Tick, false, S::Acquiring),
        (S::Acquiring, E::Tick, true, S::Releasing),
        (S::Captured, E::HotkeyPressed, false, S::Releasing),
        (S::Captured, E::CaptureConfirmed, false, S::Captured),
        (S::Captured, E::CaptureFailed, false, S::Releasing),
        (S::Captured, E::ReleaseConfirmed, false, S::Captured),
        (S::Captured, E::FocusLost, false, S::Releasing),
        (S::Captured, E::OverlayLost, false, S::Releasing),
        (S::Captured, E::ForcedRelease, false, S::Released),
        (S::Captured, E::Tick, false, S::Captured),
        (S::Captured, E::Tick, true, S::Captured),
        (S::Releasing, E::HotkeyPressed, false, S::Releasing),
        (S::Releasing, E::CaptureConfirmed, false, S::Releasing),
        (S::Releasing, E::CaptureFailed, false, S::Releasing),
        (S::Releasing, E::ReleaseConfirmed, false, S::Released),
        (S::Releasing, E::FocusLost, false, S::Releasing),
        (S::Releasing, E::OverlayLost, false, S::Releasing),
        (S::Releasing, E::ForcedRelease, false, S::Released),
        (S::Releasing, E::Tick, false, S::Releasing),
        (S::Releasing, E::Tick, true, S::Stuck),
        (S::Stuck, E::HotkeyPressed, false, S::Stuck),
        (S::Stuck, E::CaptureConfirmed, false, S::Stuck),
        (S::Stuck, E::CaptureFailed, false, S::Stuck),
        (S::Stuck, E::ReleaseConfirmed, false, S::Stuck),
        (S::Stuck, E::FocusLost, false, S::Stuck),
        (S::Stuck, E::OverlayLost, false, S::Stuck),
        (S::Stuck, E::ForcedRelease, false, S::Released),
        (S::Stuck, E::Tick, false, S::Stuck),
        (S::Stuck, E::Tick, true, S::Stuck),
    ];

    #[test]
    fn every_state_and_event_pair_lands_where_the_table_says() {
        for &(state, event, expired, expected) in TABLE {
            let elapsed = if expired { PAST_ANY_DEADLINE } else { 0 };
            assert_eq!(
                next(state, event, elapsed),
                expected,
                "{state:?} + {event:?} with elapsed {elapsed} should reach {expected:?}"
            );
        }
    }

    #[test]
    fn the_table_covers_every_state_and_event_pair() {
        // Without this the table above could silently lose a row and the test
        // that reads it would still pass.
        for state in S::ALL {
            for event in E::ALL {
                let covered = TABLE.iter().any(|&(s, e, _, _)| s == state && e == event);
                assert!(covered, "the table has no row for {state:?} + {event:?}");
            }
        }
    }

    // --- the guarantee ----------------------------------------------------

    /// Walks the machine over every event and both sides of every deadline,
    /// collecting the states that can actually occur.
    fn reachable_states() -> HashSet<S> {
        let mut seen: HashSet<S> = HashSet::new();
        let mut queue: VecDeque<S> = VecDeque::new();
        seen.insert(S::Released);
        queue.push_back(S::Released);

        while let Some(state) = queue.pop_front() {
            for event in E::ALL {
                for elapsed in [0, PAST_ANY_DEADLINE] {
                    let to = next(state, event, elapsed);
                    if seen.insert(to) {
                        queue.push_back(to);
                    }
                }
            }
        }
        seen
    }

    #[test]
    fn every_state_is_reachable_from_released() {
        // The guarantee below is stated over the reachable set, so a state that
        // turned out to be unreachable would make that test quietly weaker.
        let reachable = reachable_states();
        for state in S::ALL {
            assert!(reachable.contains(&state), "{state:?} is unreachable");
        }
    }

    /// Whether released is reachable from `state` using only the events the
    /// supervisor can raise on its own.
    ///
    /// A route out that runs through a confirmation is not a route out, because
    /// the party that would send the confirmation is the party suspected of
    /// having hung. That restriction is what makes this worth asserting.
    fn escapes_unaided(state: S) -> bool {
        let mut seen: HashSet<S> = HashSet::from([state]);
        let mut queue: VecDeque<S> = VecDeque::from([state]);

        while let Some(current) = queue.pop_front() {
            if current == S::Released {
                return true;
            }
            for event in E::UNILATERAL {
                for elapsed in [0, PAST_ANY_DEADLINE] {
                    let to = next(current, event, elapsed);
                    if seen.insert(to) {
                        queue.push_back(to);
                    }
                }
            }
        }
        false
    }

    #[test]
    fn from_every_reachable_state_the_supervisor_alone_can_get_input_back_to_the_game() {
        // This is the whole crate.
        for state in reachable_states() {
            assert!(
                escapes_unaided(state),
                "{state:?} has no route back to released using only Tick and ForcedRelease"
            );
        }
    }

    #[test]
    fn a_forced_release_reaches_released_in_one_step_from_anywhere() {
        for state in S::ALL {
            for elapsed in [0, PAST_ANY_DEADLINE] {
                assert_eq!(
                    next(state, E::ForcedRelease, elapsed),
                    S::Released,
                    "forced release did not escape {state:?}"
                );
            }
        }
    }

    #[test]
    fn no_sequence_of_five_events_leaves_input_captured_with_no_route_out() {
        // The breadth-first search above already covers the reachable set, so
        // this is redundant by construction. It is kept because it fails in a
        // different way: it names the exact sequence that broke the invariant
        // rather than only the state, which is what a debugging session needs.
        let alphabet: Vec<(E, Millis)> = E::ALL
            .iter()
            .flat_map(|&e| [(e, 0), (e, PAST_ANY_DEADLINE)])
            .collect();
        let escaping: HashSet<S> = S::ALL.into_iter().filter(|&s| escapes_unaided(s)).collect();

        fn walk(
            machine: Machine,
            depth: u32,
            alphabet: &[(E, Millis)],
            trail: &mut Vec<(E, Millis)>,
            escaping: &HashSet<S>,
        ) {
            assert!(
                escaping.contains(&machine.state()),
                "{:?} was reached by {trail:?} and has no route back to released",
                machine.state()
            );
            if depth == 0 {
                return;
            }
            for &(event, advance) in alphabet {
                let mut branch = machine;
                branch.on(event, branch.entered_at.saturating_add(advance));
                trail.push((event, advance));
                walk(branch, depth - 1, alphabet, trail, escaping);
                trail.pop();
            }
        }

        walk(Machine::new(0), 5, &alphabet, &mut Vec::new(), &escaping);
    }

    #[test]
    fn a_long_pseudo_random_walk_never_holds_input_without_a_way_back() {
        // A deterministic generator rather than a property-testing dependency:
        // the seed is in the source, so a failure is reproducible by reading it
        // rather than by copying a seed out of a log.
        let escaping: HashSet<S> = S::ALL.into_iter().filter(|&s| escapes_unaided(s)).collect();
        let mut seed: u64 = 0x5741_5443_4844_4f47;
        let mut machine = Machine::new(0);
        let mut now: Millis = 0;
        let mut visited: HashSet<S> = HashSet::new();

        for step in 0..200_000u32 {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let event = E::ALL[(seed >> 33) as usize % E::ALL.len()];
            // A spread that straddles both deadlines, so the walk crosses them
            // in both directions rather than always stepping over or short.
            now = now.saturating_add((seed >> 17) % 1_200);

            machine.on(event, now);
            visited.insert(machine.state());

            assert!(
                escaping.contains(&machine.state()),
                "step {step} left the machine in {:?} with no way back",
                machine.state()
            );
        }

        // Without this the walk could pass by never leaving released.
        assert_eq!(visited.len(), S::ALL.len(), "the walk missed a state");
    }

    // --- the failure modes the guarantee is written against ----------------

    #[test]
    fn the_hotkey_pressed_twice_quickly_ends_released_rather_than_captured() {
        // The second press arrives before the acquire has been confirmed. If
        // the machine treated it as a fresh toggle from a settled state it
        // would try to acquire again and leave the overlay holding input the
        // user has already asked it to give up.
        let mut machine = Machine::new(0);
        assert_eq!(machine.on(E::HotkeyPressed, 0).to, S::Acquiring);
        assert_eq!(machine.on(E::HotkeyPressed, 10).to, S::Releasing);
        assert_eq!(machine.on(E::ReleaseConfirmed, 20).to, S::Released);
    }

    #[test]
    fn a_confirmation_that_arrives_after_the_second_press_does_not_recapture() {
        // The dangerous ordering: press, press, and only then the first
        // acquire's confirmation lands. Honouring it would put the machine back
        // into captured with nothing on screen asking for it.
        let mut machine = Machine::new(0);
        machine.on(E::HotkeyPressed, 0);
        machine.on(E::HotkeyPressed, 10);
        assert_eq!(machine.on(E::CaptureConfirmed, 20).to, S::Releasing);
        assert_eq!(machine.on(E::ReleaseConfirmed, 30).to, S::Released);
    }

    #[test]
    fn losing_focus_while_captured_hands_input_back_without_being_asked() {
        let mut machine = Machine::new(0);
        machine.on(E::HotkeyPressed, 0);
        machine.on(E::CaptureConfirmed, 10);
        assert_eq!(machine.state(), S::Captured);

        let transition = machine.on(E::FocusLost, 20);
        assert_eq!(transition.to, S::Releasing);
        assert_eq!(transition.effect, Effect::Release);
        assert_eq!(machine.on(E::ReleaseConfirmed, 30).to, S::Released);
    }

    #[test]
    fn losing_focus_during_an_acquire_also_hands_input_back() {
        // Alt-tab landing between the swallow starting and the overlay coming
        // to the front. The window is already in the input path, so this owes a
        // release rather than a jump straight to released.
        let mut machine = Machine::new(0);
        machine.on(E::HotkeyPressed, 0);
        let transition = machine.on(E::FocusLost, 5);
        assert_eq!(transition.to, S::Releasing);
        assert_eq!(transition.effect, Effect::Release);
    }

    #[test]
    fn an_overlay_that_dies_while_capturing_does_not_keep_the_input() {
        let mut machine = Machine::new(0);
        machine.on(E::HotkeyPressed, 0);
        machine.on(E::CaptureConfirmed, 10);
        assert_eq!(machine.on(E::OverlayLost, 20).to, S::Releasing);
        assert_eq!(machine.on(E::ReleaseConfirmed, 30).to, S::Released);
    }

    #[test]
    fn an_overlay_that_hangs_escalates_to_a_forced_release() {
        // Nothing answers after the overlay is declared lost, so the release
        // deadline is the only thing that moves. It has to end somewhere the
        // supervisor can act rather than in a silent wait.
        let mut machine = Machine::new(0);
        machine.on(E::HotkeyPressed, 0);
        machine.on(E::CaptureConfirmed, 10);
        machine.on(E::OverlayLost, 20);

        let transition = machine.on(E::Tick, 20 + RELEASE_DEADLINE_MS);
        assert_eq!(transition.to, S::Stuck);
        assert_eq!(transition.effect, Effect::ForceRelease);

        let forced = machine.on(E::ForcedRelease, 2_000);
        assert_eq!(forced.to, S::Released);
        assert!(!machine.holds_input());
    }

    #[test]
    fn an_acquire_that_is_never_confirmed_times_out_toward_the_game() {
        let mut machine = Machine::new(0);
        machine.on(E::HotkeyPressed, 0);
        assert_eq!(
            machine.on(E::Tick, ACQUIRE_DEADLINE_MS - 1).to,
            S::Acquiring
        );
        assert_eq!(machine.on(E::Tick, ACQUIRE_DEADLINE_MS).to, S::Releasing);
    }

    #[test]
    fn a_stream_of_events_during_a_release_cannot_postpone_the_deadline() {
        // The bug this guards: refreshing the entry time on a self-loop. A game
        // that changes focus every frame would then hold the release open
        // forever and the escalation would never run.
        let mut machine = Machine::new(0);
        machine.on(E::HotkeyPressed, 0);
        machine.on(E::CaptureConfirmed, 1);
        machine.on(E::HotkeyPressed, 2);
        assert_eq!(machine.state(), S::Releasing);

        for t in 3..RELEASE_DEADLINE_MS + 2 {
            machine.on(E::FocusLost, t);
        }
        assert_eq!(machine.state(), S::Releasing);
        assert_eq!(machine.on(E::Tick, RELEASE_DEADLINE_MS + 2).to, S::Stuck);
    }

    #[test]
    fn while_stuck_the_tick_keeps_asking_for_the_forced_release() {
        // A supervisor whose first teardown attempt failed needs a second one,
        // and the only thing guaranteed to keep arriving is the tick.
        let mut machine = machine_in(S::Stuck);
        for at in [5_000, 5_100, 60_000] {
            let transition = machine.on(E::Tick, at);
            assert_eq!(transition.to, S::Stuck);
            assert_eq!(transition.effect, Effect::ForceRelease);
        }
    }

    #[test]
    fn while_stuck_an_event_that_is_not_a_tick_does_not_re_ask_for_the_teardown() {
        // The livelock this guards against, found by running the example: a
        // caller that answers each request by scheduling work a moment later
        // has that work pushed back by every stray event, and the teardown
        // never happens. The retry belongs to the tick, whose rate the caller
        // controls.
        let mut machine = machine_in(S::Stuck);
        for event in E::ALL {
            if matches!(event, E::Tick | E::ForcedRelease) {
                continue;
            }
            let transition = machine.on(event, 5_000);
            assert_eq!(transition.to, S::Stuck);
            assert_eq!(
                transition.effect,
                Effect::None,
                "{event:?} while stuck re-asked for the teardown"
            );
        }
    }

    #[test]
    fn a_release_that_confirms_late_does_not_get_the_machine_out_of_stuck() {
        // Deliberate. Past the deadline the overlay's word is not evidence, and
        // accepting it would let a hung process talk its way back into being
        // trusted with the player's input.
        let mut machine = machine_in(S::Stuck);
        assert_eq!(machine.on(E::ReleaseConfirmed, 9_000).to, S::Stuck);
        assert_eq!(machine.on(E::ForcedRelease, 9_100).to, S::Released);
    }

    #[test]
    fn a_failed_acquire_still_runs_the_release_path() {
        // Bringing the overlay forward failed, but the filter was already
        // swallowing by then, so declaring the game safe here would be a guess.
        let mut machine = Machine::new(0);
        machine.on(E::HotkeyPressed, 0);
        let transition = machine.on(E::CaptureFailed, 5);
        assert_eq!(transition.to, S::Releasing);
        assert_eq!(transition.effect, Effect::Release);
    }

    #[test]
    fn released_is_a_sink_for_everything_except_the_hotkey() {
        let mut machine = Machine::new(0);
        for event in E::ALL {
            if event == E::HotkeyPressed {
                continue;
            }
            let transition = machine.on(event, 100_000);
            assert_eq!(transition.to, S::Released, "{event:?} disturbed released");
            assert_eq!(transition.effect, Effect::None);
        }
    }

    // --- the lease --------------------------------------------------------

    #[test]
    fn a_capture_that_is_not_refreshed_stops_swallowing_on_its_own() {
        // The case the watchdog cannot cover: the supervisor stopped running,
        // so nothing is left to notice that anything is wrong. The filter's own
        // clock has to be what ends it.
        let mut machine = Machine::new(0);
        machine.on(E::HotkeyPressed, 0);
        machine.on(E::CaptureConfirmed, 10);

        let capture = machine.capture(1_000);
        assert!(capture.swallows(1_000));
        assert!(capture.swallows(1_000 + CAPTURE_LEASE_MS - 1));
        assert!(!capture.swallows(1_000 + CAPTURE_LEASE_MS));
        assert!(!capture.swallows(60_000));
    }

    #[test]
    fn a_released_capture_never_swallows_at_any_reading() {
        let machine = Machine::new(0);
        for now in [0, 1, 999, 100_000] {
            assert!(!machine.capture(now).swallows(now));
        }
        assert!(!Capture::released().swallows(0));
    }

    #[test]
    fn every_holding_state_publishes_a_capture_that_swallows_while_it_is_fresh() {
        for state in S::ALL {
            let machine = machine_in(state);
            let capture = machine.capture(5_000);
            assert_eq!(
                capture.swallows(5_000),
                state.holds_input(),
                "{state:?} published the wrong capture"
            );
        }
    }

    #[test]
    fn a_clock_that_goes_backwards_does_not_expire_a_deadline_early() {
        // Not hypothetical: a machine feeding this from two threads with
        // separately read clocks can hand it a reading older than the last one.
        let mut machine = Machine::new(1_000);
        machine.on(E::HotkeyPressed, 1_000);
        assert_eq!(machine.elapsed(500), 0);
        assert_eq!(machine.on(E::Tick, 500).to, S::Acquiring);
    }

    // --- effects ----------------------------------------------------------

    #[test]
    fn entering_a_transient_state_asks_for_the_matching_platform_call() {
        let mut machine = Machine::new(0);
        assert_eq!(machine.on(E::HotkeyPressed, 0).effect, Effect::Acquire);
        assert_eq!(machine.on(E::HotkeyPressed, 1).effect, Effect::Release);
    }

    #[test]
    fn staying_in_a_transient_state_does_not_repeat_the_platform_call() {
        // Re-issuing the release on every stray event would restart work that
        // is already in flight, and the deadline is what catches a release that
        // never lands, not a retry loop.
        let mut machine = Machine::new(0);
        machine.on(E::HotkeyPressed, 0);
        machine.on(E::HotkeyPressed, 1);
        assert_eq!(machine.state(), S::Releasing);
        assert_eq!(machine.on(E::FocusLost, 2).effect, Effect::None);
        assert_eq!(machine.on(E::CaptureConfirmed, 3).effect, Effect::None);
    }

    #[test]
    fn a_transition_reports_whether_anything_actually_moved() {
        let mut machine = Machine::new(0);
        assert!(machine.on(E::HotkeyPressed, 0).changed());
        assert!(!machine.on(E::Tick, 1).changed());
    }
}
