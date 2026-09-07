//! Hotkeys, and the guarantee that input goes back to the game.
//!
//! The failure this crate is written against is not a crash. It is a player
//! holding W and watching their character stand still, with no reason to blame
//! anything but the game. A crash at least says something happened. Input that
//! never comes back looks exactly like the game breaking, and a player who has
//! that happen once does not run the overlay again.
//!
//! So the design question is not "how does the overlay take input", which is
//! easy, but "what has to be true for the overlay to be unable to keep it".
//!
//! # The shape of the thing
//!
//! Three parties, and the split between them is load-bearing:
//!
//! - The **supervisor** is the Stasis process, outside the game. It owns the
//!   hotkey registration, runs the [`Machine`], and publishes a [`Capture`].
//! - The **filter** is inside the game process, in the hook. It reads the
//!   published [`Capture`] and decides whether to swallow an input message.
//! - The **overlay** is the thing being toggled, and is the part that can hang.
//!
//! The hotkey is registered by the supervisor, not by the hook. Windows takes a
//! registered combination out of the input queue before the game's window
//! procedure ever sees the key, so a hook that has stopped running cannot eat
//! the key that turns it off. A toggle the overlay reads for itself would fail
//! precisely when it is needed.
//!
//! # Capture is a lease, not a latch
//!
//! This is the part worth arguing about, so here is the argument.
//!
//! The obvious design is a flag: the supervisor sets "swallow" and later clears
//! it. Every safeguard on top of that design has the same shape, which is that
//! some healthy party notices trouble and clears the flag. A watchdog clears
//! it. A focus-loss handler clears it. A process-exit handler clears it. All
//! three are actions taken by a supervisor that is still working, and all three
//! fail in the one case that matters most: the supervisor itself is the thing
//! that broke. Nothing is left to notice, and the flag stays set.
//!
//! So the flag carries an expiry instead. [`Machine::capture`] returns a
//! [`Capture`] whose `expires_at` is [`CAPTURE_LEASE_MS`] into the future, the
//! supervisor republishes it on every loop, and the filter calls
//! [`Capture::swallows`] against *its own* clock. A supervisor that has stopped
//! running stops refreshing, and input goes back to the game about a second
//! later without anybody having to do anything. Releasing becomes the absence
//! of an action rather than the presence of one, which is the only version of
//! this that survives its own machinery dying.
//!
//! Two rules follow for whoever writes the filter, and they are not optional:
//!
//! - Read the lease with the filter's clock, never a timestamp handed over with
//!   the capture. A stale clock and a stale flag fail together.
//! - Anything ambiguous passes the message through. There is no state of this
//!   system in which swallowing is the safe guess.
//!
//! # What the state machine is for
//!
//! [`Machine`] is pure: no clock, no Windows, no I/O. It takes an
//! [`InputEvent`] and a millisecond reading and returns a [`Transition`] naming
//! the [`Effect`] the caller now owes. That is what makes the guarantee
//! checkable rather than argued: the tests enumerate every state, every event,
//! both sides of every deadline, and assert that from every reachable state
//! there is a path back to [`InputState::Released`] using only the events the
//! supervisor can raise on its own.
//!
//! The restriction on that last part is the point. A route home that runs
//! through [`InputEvent::ReleaseConfirmed`] is not a route home, because the
//! party that sends confirmations is the party suspected of being hung.
//!
//! # What this crate does not do
//!
//! It does not send input. Reading input and deciding whether to swallow it is
//! the job; generating input into a game is a cheat capability, it is one of
//! the things `CONTRIBUTING.md` promises this project will not add, and no code
//! here or downstream of here should ever call `SendInput`.
//!
//! It also does not clip or confine the cursor, and the filter should not
//! either. `ClipCursor` is process-wide and outlives the supervisor that asked
//! for it, so a confined cursor is a captured input path that the lease cannot
//! expire.
//!
//! # Using it
//!
//! ```no_run
//! use stasis_input::{HotkeyRegistration, InputEvent, Machine, DEFAULT_HOTKEY};
//!
//! let registration = HotkeyRegistration::register(DEFAULT_HOTKEY)?;
//! let mut machine = Machine::new(0);
//!
//! // Driven from the supervisor's message pump, with `now` read from a
//! // monotonic clock. The tick has to keep arriving even when nothing else
//! // does, because the deadlines are what catch a release that never lands.
//! let transition = machine.on(InputEvent::HotkeyPressed, 16);
//! let capture = machine.capture(16);
//! # Ok::<(), stasis_input::HotkeyError>(())
//! ```

#![deny(missing_docs)]

mod hotkey;
mod machine;

pub use hotkey::{Hotkey, HotkeyError, Key, Modifiers, ParseHotkeyError, DEFAULT_HOTKEY};
pub use machine::{
    Capture, Effect, InputEvent, InputState, Machine, Transition, ACQUIRE_DEADLINE_MS,
    CAPTURE_LEASE_MS, RELEASE_DEADLINE_MS,
};

#[cfg(windows)]
mod registrar;

#[cfg(windows)]
pub use registrar::HotkeyRegistration;

/// A monotonic reading in milliseconds.
///
/// The origin does not matter and is never interpreted, only subtracted, so any
/// steady clock works. It is passed in rather than read here so the machine
/// stays pure and the deadline tests can cover hours without waiting for them.
pub type Millis = u64;
