//! Finds a running game, the window to draw over, and what it renders with.
//!
//! This is the first thing the engine does and the easiest place to do harm.
//! Attaching to the wrong window is worse than attaching to none, so the shape
//! of this crate follows from that: the Win32 enumeration is separated from
//! every decision it feeds, the decisions are pure functions over plain
//! structs in [`window`], [`display`], [`presentation`], [`graphics`] and
//! [`lifecycle`], and each is tested against the cases that turn up. A splash
//! screen, a console window belonging to the same process, a cloaked window
//! that a suspended app left behind, and a game on Vulkan are all things this
//! crate has to get right without a game to try it on.
//!
//! Two rules run through all of it.
//!
//! **No handle to the game.** A process handle is something an anti-cheat can
//! see, and an overlay that opens one with read access looks like a memory
//! cheat. The process id comes from the window, the executable name and the
//! loaded module list come from Toolhelp snapshots, and nothing here calls
//! `OpenProcess`. Where that costs an answer, the answer is reported as
//! unknown.
//!
//! **Absent is absent.** A presentation mode that cannot be told apart from
//! another comes back as `None`, a module list that could not be read is not an
//! empty one, and a monitor whose DPI is unreadable is reported as no monitor
//! rather than as a monitor at the default scale.
//!
//! ```no_run
//! use stasis_detect::{find, GameTarget};
//!
//! // The harness in `engine/harness` registers this class. A real game would
//! // be matched by executable name from the library the app already has.
//! if let Some(game) = find(&GameTarget::WindowClass("StasisHarnessWindow".into())) {
//!     println!("{} on {}", game.window.title(), game.support());
//! }
//! ```

#![deny(missing_docs)]

pub mod display;
pub mod graphics;
pub mod lifecycle;
pub mod presentation;
pub mod window;

#[cfg(windows)]
pub mod platform;

pub use display::{display_change, DisplayChange, DpiAwareness, Monitor};
pub use graphics::{GraphicsApi, GraphicsProfile, RendererSupport};
pub use lifecycle::{Lifecycle, LifecycleAction, LifecycleEvent, LifecycleState, Transition};
pub use presentation::{ExclusiveFullscreenHint, PresentationMode};
pub use window::{Rect, WindowFacts, WindowRejection};

/// What the caller knows about the game it is looking for.
///
/// The engine does not keep its own list of games. The app already has one,
/// from the Steam library and the launchers it reads, so the target comes from
/// there and this crate does the finding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameTarget {
    /// A process id, for a game the app started or already found.
    Pid(u32),
    /// An executable file name with no path, matched case insensitively, such
    /// as `hl2.exe`.
    Executable(String),
    /// A window class name, matched case insensitively. Useful for a game whose
    /// launcher and executable names are not stable but whose render window
    /// class is.
    WindowClass(String),
}

/// The window the overlay would draw over, and what was decided about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderWindow {
    /// Everything read from the window itself.
    pub facts: WindowFacts,
    /// Windowed, borderless or exclusive fullscreen, or `None` when the two
    /// fullscreen modes could not be told apart. See
    /// [`presentation::presentation_mode`] for when that happens and why it is
    /// not guessed.
    pub presentation: Option<PresentationMode>,
}

impl RenderWindow {
    /// The raw `HWND` value. Rebuild the handle with `HWND(value)`.
    pub fn hwnd(&self) -> isize {
        self.facts.hwnd
    }

    /// The window title, which is empty for many borderless game windows.
    pub fn title(&self) -> &str {
        &self.facts.title
    }

    /// The registered window class.
    pub fn class_name(&self) -> &str {
        &self.facts.class_name
    }

    /// The size of the surface to draw into, when it can be measured.
    pub fn size(&self) -> Option<(i32, i32)> {
        self.facts.drawable_size()
    }

    /// The display the window is on, absent while it is minimised or off every
    /// monitor.
    pub fn monitor(&self) -> Option<&Monitor> {
        self.facts.monitor.as_ref()
    }

    /// True when a separate always on top window would be visible over this
    /// one. `None` when the presentation mode is not known, which is not the
    /// same as false.
    pub fn allows_composited_overlay(&self) -> Option<bool> {
        self.presentation
            .map(|mode| mode.allows_composited_overlay())
    }
}

/// A game the detector found, and everything decided about it in one pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetectedGame {
    /// The process id, taken from the window rather than from a handle.
    pub pid: u32,
    /// The executable file name, or `None` when the process snapshot could not
    /// be read. The window is enough to attach to, so this is a label rather
    /// than a requirement.
    pub executable: Option<String>,
    /// The window to draw over.
    pub window: RenderWindow,
    /// The graphics runtimes the process has loaded.
    pub graphics: GraphicsProfile,
}

impl DetectedGame {
    /// What the engine can do about this game's renderer.
    ///
    /// This is the value that lets the app say "this game uses Vulkan, which is
    /// not supported yet" instead of showing nothing.
    pub fn support(&self) -> RendererSupport {
        self.graphics.support()
    }
}

#[cfg(windows)]
mod detection {
    use super::{DetectedGame, GameTarget, RenderWindow};
    use crate::platform;
    use crate::presentation::presentation_mode;
    use crate::window::{choose_render_window, WindowFacts};

    /// Find one game matching the target, or report that none is running.
    ///
    /// Returns `None` both when no process matches and when a process matches
    /// but none of its windows is something to draw over. The two are told
    /// apart by [`windows_of`] plus [`crate::window::rejection`], which is what
    /// a caller should reach for when it wants to explain a failure to a user
    /// rather than only react to it.
    pub fn find(target: &GameTarget) -> Option<DetectedGame> {
        find_all(target).into_iter().next()
    }

    /// Every game matching the target, largest render window first.
    ///
    /// More than one is normal: a game with a launcher that stays running, or
    /// two copies of the same executable.
    pub fn find_all(target: &GameTarget) -> Vec<DetectedGame> {
        let hint = platform::exclusive_fullscreen_hint();
        let windows = platform::all_windows();

        let mut pids: Vec<u32> = match target {
            GameTarget::Pid(pid) => vec![*pid],
            // Matching on the class needs no process snapshot at all, which
            // makes it the quietest of the three.
            GameTarget::WindowClass(class) => windows
                .iter()
                .filter(|window| window.class_name.eq_ignore_ascii_case(class))
                .map(|window| window.pid)
                .collect(),
            GameTarget::Executable(name) => platform::processes_named(name)
                .into_iter()
                .map(|process| process.pid)
                .collect(),
        };
        pids.sort_unstable();
        pids.dedup();

        let mut found: Vec<DetectedGame> = pids
            .into_iter()
            .filter_map(|pid| {
                let of_process: Vec<WindowFacts> = windows
                    .iter()
                    .filter(|window| window.pid == pid)
                    .cloned()
                    .collect();
                let facts = choose_render_window(&of_process)?.clone();

                Some(DetectedGame {
                    pid,
                    executable: platform::process_name(pid),
                    window: RenderWindow {
                        presentation: presentation_mode(&facts, hint),
                        facts,
                    },
                    graphics: platform::graphics_profile(pid),
                })
            })
            .collect();

        // Largest first, for the same reason the window ranking uses size: the
        // biggest surface is the one being played.
        found.sort_by_key(|game| {
            std::cmp::Reverse(
                game.window
                    .size()
                    .map(|(w, h)| i64::from(w) * i64::from(h))
                    .unwrap_or(0),
            )
        });
        found
    }

    /// Every top-level window of a process, accepted or not.
    ///
    /// For a caller that wants to show why a window was skipped. Pair it with
    /// [`crate::window::rejection`].
    pub fn windows_of(pid: u32) -> Vec<WindowFacts> {
        platform::windows_of_process(pid)
    }
}

#[cfg(windows)]
pub use detection::{find, find_all, windows_of};

#[cfg(windows)]
pub use platform::{
    adopt_per_monitor_dpi_awareness, dpi_awareness, exclusive_fullscreen_hint, graphics_profile,
    process_name, processes, processes_named, ProcessEntry,
};
