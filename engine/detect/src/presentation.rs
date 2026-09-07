//! Exclusive fullscreen, borderless, or windowed, decided from outside the
//! game process.
//!
//! This is the value that decides whether an overlay can appear at all, so it
//! is worth being clear about how much of it is knowable from outside. Geometry
//! and window style separate windowed from fullscreen with certainty. They
//! cannot separate exclusive fullscreen from borderless, because a borderless
//! game and an exclusive one look identical from the window manager's side. The
//! shell knows, because it stops showing notifications over an exclusive
//! Direct3D application, and that answer is passed in here as a hint.
//!
//! When the hint is not available the mode is reported as absent rather than
//! guessed. Guessing borderless would tell the app an overlay will appear when
//! it may not, which is the exact failure this crate exists to avoid.

use crate::window::WindowFacts;

/// How a window is being presented.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationMode {
    /// A window with a frame, or one that does not cover its monitor. Includes
    /// a maximised window, which is windowed with the frame at the edges.
    Windowed,
    /// Frameless and exactly covering its monitor. The desktop compositor is
    /// still in the path.
    Borderless,
    /// The game has taken the display. The compositor is out of the path, and
    /// the mode switch is real rather than a resize.
    Exclusive,
}

impl std::fmt::Display for PresentationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PresentationMode::Windowed => write!(f, "windowed"),
            PresentationMode::Borderless => write!(f, "borderless fullscreen"),
            PresentationMode::Exclusive => write!(f, "exclusive fullscreen"),
        }
    }
}

impl PresentationMode {
    /// True when a separate always on top window drawn by the desktop
    /// compositor would be visible over the game.
    ///
    /// False for exclusive fullscreen: the game owns the scanout, the
    /// compositor is not composing anything over it, and a second window would
    /// either be invisible or force the game out of its mode. That case needs
    /// the hooked path, and an app that cannot take it has to say so instead of
    /// showing an overlay that never appears.
    pub fn allows_composited_overlay(&self) -> bool {
        !matches!(self, PresentationMode::Exclusive)
    }

    /// True when the window covers its whole monitor, either way of doing it.
    pub fn is_fullscreen(&self) -> bool {
        !matches!(self, PresentationMode::Windowed)
    }
}

/// What the shell says about exclusive fullscreen, which is the one signal that
/// separates it from borderless.
///
/// `SHQueryUserNotificationState` answers for the session rather than for one
/// window, so it is only trusted about a window that is in the foreground.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExclusiveFullscreenHint {
    /// The shell reports a Direct3D application running in exclusive mode.
    Running,
    /// The shell reports a state that is not exclusive Direct3D.
    NotRunning,
    /// The query failed or was not made.
    Unknown,
}

/// Decide the presentation mode of a window.
///
/// Returns `None` in the three cases where the answer is genuinely not
/// available: while the window is minimised, when the window is on no monitor
/// so there is nothing to compare its size against, and when the window is
/// fullscreen shaped but the shell hint that would separate exclusive from
/// borderless is missing.
pub fn presentation_mode(
    facts: &WindowFacts,
    hint: ExclusiveFullscreenHint,
) -> Option<PresentationMode> {
    // A minimised window is parked off screen at an icon's size. Its restore
    // rectangle says where it will go, not how it will present when it gets
    // there, so the mode is not knowable until it is back.
    if facts.minimized {
        return None;
    }

    let monitor = facts.monitor.as_ref()?;

    // A frame means windowed, whatever the size. A maximised window covers the
    // work area and keeps its caption, and games in exclusive or borderless
    // fullscreen have neither caption nor resize border.
    if facts.caption || facts.resizable {
        return Some(PresentationMode::Windowed);
    }

    let bounds = facts.bounds;
    let screen = monitor.bounds;
    let covers_monitor = bounds.left <= screen.left
        && bounds.top <= screen.top
        && bounds.right >= screen.right
        && bounds.bottom >= screen.bottom
        && !screen.is_empty();

    if !covers_monitor {
        return Some(PresentationMode::Windowed);
    }

    match hint {
        // The hint is about the session, so it only settles the question for
        // the window the session is actually showing.
        ExclusiveFullscreenHint::Running if facts.foreground => Some(PresentationMode::Exclusive),
        ExclusiveFullscreenHint::Running => Some(PresentationMode::Borderless),
        ExclusiveFullscreenHint::NotRunning => Some(PresentationMode::Borderless),
        ExclusiveFullscreenHint::Unknown => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display::Monitor;
    use crate::window::Rect;

    const SCREEN: Rect = Rect {
        left: 0,
        top: 0,
        right: 1920,
        bottom: 1080,
    };

    fn windowed() -> WindowFacts {
        WindowFacts {
            hwnd: 0x1000,
            bounds: Rect::new(100, 100, 1280, 720),
            client: Rect::new(0, 0, 1264, 681),
            visible: true,
            caption: true,
            resizable: true,
            foreground: true,
            monitor: Some(Monitor::new(SCREEN, 96)),
            ..Default::default()
        }
    }

    fn fullscreen_shaped() -> WindowFacts {
        WindowFacts {
            caption: false,
            resizable: false,
            bounds: SCREEN,
            client: Rect::new(0, 0, 1920, 1080),
            ..windowed()
        }
    }

    #[test]
    fn a_framed_window_is_windowed() {
        assert_eq!(
            presentation_mode(&windowed(), ExclusiveFullscreenHint::NotRunning),
            Some(PresentationMode::Windowed)
        );
    }

    #[test]
    fn a_maximised_window_is_still_windowed() {
        // It covers the monitor and keeps its caption. Reporting fullscreen
        // here would tell the app the compositor is out of the path when the
        // player is one alt-tab away from a task bar.
        let mut window = windowed();
        window.bounds = SCREEN;
        window.client = Rect::new(0, 0, 1920, 1040);

        assert_eq!(
            presentation_mode(&window, ExclusiveFullscreenHint::NotRunning),
            Some(PresentationMode::Windowed)
        );
    }

    #[test]
    fn a_frameless_window_covering_its_monitor_is_borderless() {
        assert_eq!(
            presentation_mode(&fullscreen_shaped(), ExclusiveFullscreenHint::NotRunning),
            Some(PresentationMode::Borderless)
        );
    }

    #[test]
    fn a_borderless_window_slightly_larger_than_the_monitor_still_counts() {
        // Some engines round the window out by a pixel or overhang a monitor
        // whose bounds are not at the origin. Requiring an exact match would
        // report those as windowed.
        let mut window = fullscreen_shaped();
        window.bounds = Rect::new(-1, -1, 1922, 1082);

        assert_eq!(
            presentation_mode(&window, ExclusiveFullscreenHint::NotRunning),
            Some(PresentationMode::Borderless)
        );
    }

    #[test]
    fn a_frameless_window_smaller_than_its_monitor_is_windowed() {
        let mut window = fullscreen_shaped();
        window.bounds = Rect::new(0, 0, 1280, 720);

        assert_eq!(
            presentation_mode(&window, ExclusiveFullscreenHint::NotRunning),
            Some(PresentationMode::Windowed)
        );
    }

    #[test]
    fn the_shell_reporting_exclusive_direct3d_makes_a_foreground_window_exclusive() {
        assert_eq!(
            presentation_mode(&fullscreen_shaped(), ExclusiveFullscreenHint::Running),
            Some(PresentationMode::Exclusive)
        );
    }

    #[test]
    fn a_background_window_is_not_the_one_the_shell_is_talking_about() {
        // The shell state is per session. A second game fullscreen on another
        // monitor would otherwise make this one exclusive too.
        let mut window = fullscreen_shaped();
        window.foreground = false;

        assert_eq!(
            presentation_mode(&window, ExclusiveFullscreenHint::Running),
            Some(PresentationMode::Borderless)
        );
    }

    #[test]
    fn a_fullscreen_window_with_no_shell_hint_reports_no_mode_rather_than_guessing() {
        // Guessing borderless would promise an overlay that exclusive
        // fullscreen will not show.
        assert_eq!(
            presentation_mode(&fullscreen_shaped(), ExclusiveFullscreenHint::Unknown),
            None
        );
    }

    #[test]
    fn a_windowed_game_needs_no_shell_hint() {
        // The geometry settles it, so a failed shell query costs nothing here.
        assert_eq!(
            presentation_mode(&windowed(), ExclusiveFullscreenHint::Unknown),
            Some(PresentationMode::Windowed)
        );
    }

    #[test]
    fn a_minimised_window_has_no_presentation_mode() {
        let mut window = fullscreen_shaped();
        window.minimized = true;

        assert_eq!(
            presentation_mode(&window, ExclusiveFullscreenHint::Running),
            None
        );
    }

    #[test]
    fn a_window_on_no_monitor_has_no_presentation_mode() {
        let mut window = fullscreen_shaped();
        window.monitor = None;

        assert_eq!(
            presentation_mode(&window, ExclusiveFullscreenHint::Running),
            None
        );
    }

    #[test]
    fn only_exclusive_fullscreen_rules_out_a_composited_overlay() {
        assert!(PresentationMode::Windowed.allows_composited_overlay());
        assert!(PresentationMode::Borderless.allows_composited_overlay());
        assert!(!PresentationMode::Exclusive.allows_composited_overlay());
    }

    #[test]
    fn both_fullscreen_modes_report_as_fullscreen() {
        assert!(PresentationMode::Borderless.is_fullscreen());
        assert!(PresentationMode::Exclusive.is_fullscreen());
        assert!(!PresentationMode::Windowed.is_fullscreen());
    }
}
