//! What a window is, and whether it is the one to draw over.
//!
//! Everything here is pure. The facts about a window arrive as a plain struct
//! that any test can build, and the decisions are functions over that struct.
//! The Win32 calls that fill the struct in live in `platform`, because they
//! cannot be unit tested and the decisions can.
//!
//! The rule this module exists to enforce is that attaching to the wrong window
//! is worse than attaching to none, so every rejection is a named reason the
//! app can show rather than a silent `false`.

use crate::display::Monitor;

/// A rectangle in virtual desktop coordinates, laid out like Win32 `RECT`.
///
/// Right and bottom are exclusive, so a 1280 wide window at x=0 has
/// `right == 1280`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    /// Left edge, inclusive.
    pub left: i32,
    /// Top edge, inclusive.
    pub top: i32,
    /// Right edge, exclusive.
    pub right: i32,
    /// Bottom edge, exclusive.
    pub bottom: i32,
}

impl Rect {
    /// A rectangle from a position and a size, which is how tests read best.
    pub fn new(left: i32, top: i32, width: i32, height: i32) -> Self {
        Rect {
            left,
            top,
            right: left + width,
            bottom: top + height,
        }
    }

    /// Width in pixels. Negative if the rectangle is inside out, which Win32
    /// does produce for windows that have never been laid out.
    pub fn width(&self) -> i32 {
        self.right - self.left
    }

    /// Height in pixels.
    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }

    /// Area in pixels, in `i64` because a wide virtual desktop overflows `i32`.
    pub fn area(&self) -> i64 {
        i64::from(self.width().max(0)) * i64::from(self.height().max(0))
    }

    /// True when the rectangle covers no pixels at all.
    pub fn is_empty(&self) -> bool {
        self.width() <= 0 || self.height() <= 0
    }
}

/// Why a window was not accepted as the thing to draw over.
///
/// Each variant is a deliberate decision rather than a fallthrough, so the app
/// can say which window it skipped and why instead of reporting nothing found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowRejection {
    /// The window is not visible. A minimised game is still visible in the
    /// Win32 sense, so this really does mean hidden.
    Invisible,
    /// Cloaked by the shell. Suspended store apps and windows on another
    /// virtual desktop are visible by style and not on screen at all.
    Cloaked,
    /// A child window. Its pixels belong to its parent's surface.
    ChildWindow,
    /// Owned by another window, which is the shape of a dialog or a tool
    /// palette rather than a main window.
    OwnedWindow,
    /// A tool window, which the shell already excludes from the task bar for
    /// the same reason it is excluded here.
    ToolWindow,
    /// No measurable client area, and no restore rectangle to fall back on.
    NoClientArea,
    /// A console window. A game started from a terminal has one in the same
    /// window list, sometimes under the same process id.
    ConsoleWindow,
    /// Smaller than any resolution a game presents at, so it is a splash
    /// screen, a launcher, or a tooltip.
    BelowMinimumSize {
        /// Measured width in pixels.
        width: i32,
        /// Measured height in pixels.
        height: i32,
    },
}

impl std::fmt::Display for WindowRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WindowRejection::Invisible => write!(f, "not visible"),
            WindowRejection::Cloaked => write!(f, "cloaked by the shell"),
            WindowRejection::ChildWindow => write!(f, "a child window"),
            WindowRejection::OwnedWindow => write!(f, "owned by another window"),
            WindowRejection::ToolWindow => write!(f, "a tool window"),
            WindowRejection::NoClientArea => write!(f, "no client area"),
            WindowRejection::ConsoleWindow => write!(f, "a console window"),
            WindowRejection::BelowMinimumSize { width, height } => {
                write!(f, "only {width}x{height}, too small to be a game surface")
            }
        }
    }
}

/// The smallest render surface a shipping game offers, near enough. QVGA is
/// below every mode in every options menu worth naming, so anything under it is
/// something other than the game.
pub const MINIMUM_SURFACE_WIDTH: i32 = 320;
/// Height counterpart to [`MINIMUM_SURFACE_WIDTH`].
pub const MINIMUM_SURFACE_HEIGHT: i32 = 240;

/// Window classes that host a console. Matching is case insensitive because
/// Win32 registers class names that way.
///
/// `ConsoleWindowClass` is the classic console, `CASCADIA_HOSTING_WINDOW_CLASS`
/// is Windows Terminal, and `PseudoConsoleWindow` is the hidden window ConPTY
/// creates. A game launched from a shell can have any of them nearby.
const CONSOLE_CLASSES: [&str; 3] = [
    "ConsoleWindowClass",
    "CASCADIA_HOSTING_WINDOW_CLASS",
    "PseudoConsoleWindow",
];

/// Everything about one window that the decision needs, and nothing that needs
/// a live Win32 handle to read.
///
/// [`Default`] gives a window that fails every test: invisible, sizeless and
/// nameless. Tests build the case they mean by setting only the fields that
/// matter to them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WindowFacts {
    /// The raw `HWND` value. Kept as `isize` so this type does not pin the
    /// consumer to one version of the `windows` crate. Rebuild it with
    /// `HWND(hwnd)`.
    pub hwnd: isize,
    /// The owning process id, read from the window rather than from a process
    /// handle that had to be opened.
    pub pid: u32,
    /// The registered window class.
    pub class_name: String,
    /// The title bar text, often empty for a borderless game window.
    pub title: String,
    /// Outer bounds in virtual desktop coordinates.
    pub bounds: Rect,
    /// Client area, in client coordinates, so left and top are zero.
    pub client: Rect,
    /// Where the window sits when it is restored. Win32 keeps this for a
    /// minimised window, whose live rectangle is off screen and useless.
    pub restored_bounds: Option<Rect>,
    /// `IsWindowVisible`, which stays true while minimised.
    pub visible: bool,
    /// `IsIconic`.
    pub minimized: bool,
    /// The DWM cloak state, which the style bits do not report.
    pub cloaked: bool,
    /// True when this is the foreground window of the whole session.
    pub foreground: bool,
    /// `WS_CHILD`.
    pub child: bool,
    /// Has an owner window, from `GetWindow(GW_OWNER)`.
    pub owned: bool,
    /// `WS_EX_TOOLWINDOW`.
    pub tool_window: bool,
    /// `WS_CAPTION`, the title bar. A borderless fullscreen game has none.
    pub caption: bool,
    /// `WS_THICKFRAME`, the resize border.
    pub resizable: bool,
    /// The monitor the window is on, absent when the window is off every
    /// display, which happens while it is minimised.
    pub monitor: Option<Monitor>,
}

impl WindowFacts {
    /// The size of the surface the overlay would draw into, when it can be
    /// measured.
    ///
    /// A minimised window reports a client area of zero and a position out at
    /// -32000, so its restore rectangle is used instead. That is the outer
    /// rectangle rather than the client one, which overstates the surface by
    /// the frame, and at these sizes the frame does not change any decision.
    pub fn drawable_size(&self) -> Option<(i32, i32)> {
        if !self.minimized && !self.client.is_empty() {
            return Some((self.client.width(), self.client.height()));
        }
        match self.restored_bounds {
            Some(rect) if !rect.is_empty() => Some((rect.width(), rect.height())),
            _ => None,
        }
    }

    /// Area of [`WindowFacts::drawable_size`], or zero when it cannot be
    /// measured. Used for ranking, where an unmeasurable window should lose.
    fn drawable_area(&self) -> i64 {
        match self.drawable_size() {
            Some((w, h)) => i64::from(w.max(0)) * i64::from(h.max(0)),
            None => 0,
        }
    }
}

/// Why this window is not the one to draw over, or `None` when it is a
/// candidate.
///
/// The order of the checks is the order of certainty. Being invisible or
/// cloaked settles the question on its own, so those come before the shape
/// tests, and a caller showing the reason to a user gets the strongest one.
pub fn rejection(facts: &WindowFacts) -> Option<WindowRejection> {
    if !facts.visible {
        return Some(WindowRejection::Invisible);
    }
    if facts.cloaked {
        return Some(WindowRejection::Cloaked);
    }
    if facts.child {
        return Some(WindowRejection::ChildWindow);
    }
    if facts.owned {
        return Some(WindowRejection::OwnedWindow);
    }
    if facts.tool_window {
        return Some(WindowRejection::ToolWindow);
    }
    if CONSOLE_CLASSES
        .iter()
        .any(|class| class.eq_ignore_ascii_case(&facts.class_name))
    {
        return Some(WindowRejection::ConsoleWindow);
    }
    let Some((width, height)) = facts.drawable_size() else {
        return Some(WindowRejection::NoClientArea);
    };
    if width < MINIMUM_SURFACE_WIDTH || height < MINIMUM_SURFACE_HEIGHT {
        return Some(WindowRejection::BelowMinimumSize { width, height });
    }
    None
}

/// True when this window could be the one the game presents into.
pub fn is_render_target(facts: &WindowFacts) -> bool {
    rejection(facts).is_none()
}

/// Pick the window to draw over out of a list, or report that none of them
/// qualifies.
///
/// Ranking is by drawable area first. A splash screen and the game window are
/// both real windows of the same process at the same moment, and the game is
/// the larger one. Foreground breaks a tie, because two windows of the same
/// size are usually a game and its own dialog, and a window that is not
/// minimised beats one that is.
pub fn choose_render_window(facts: &[WindowFacts]) -> Option<&WindowFacts> {
    facts
        .iter()
        .filter(|window| is_render_target(window))
        .max_by(|a, b| {
            a.drawable_area()
                .cmp(&b.drawable_area())
                .then(a.foreground.cmp(&b.foreground))
                .then(b.minimized.cmp(&a.minimized))
                // Last resort, so the choice is stable across scans rather than
                // flapping between two identical windows.
                .then(b.hwnd.cmp(&a.hwnd))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game_window() -> WindowFacts {
        WindowFacts {
            hwnd: 0x1000,
            pid: 4242,
            class_name: "StasisHarnessWindow".into(),
            title: "Stasis harness (pretend this is a game)".into(),
            bounds: Rect::new(100, 100, 1280, 720),
            client: Rect::new(0, 0, 1264, 681),
            visible: true,
            caption: true,
            resizable: true,
            monitor: Some(Monitor::new(Rect::new(0, 0, 2560, 1440), 96)),
            ..Default::default()
        }
    }

    #[test]
    fn a_plain_windowed_game_is_a_render_target() {
        assert_eq!(rejection(&game_window()), None);
    }

    #[test]
    fn the_default_facts_are_rejected_rather_than_accepted_by_omission() {
        // Default exists for test ergonomics. If it ever passed the checks, a
        // test that forgot a field would be asserting nothing.
        assert!(rejection(&WindowFacts::default()).is_some());
    }

    #[test]
    fn a_window_with_no_client_area_is_not_a_render_target() {
        let mut window = game_window();
        window.client = Rect::default();
        assert_eq!(rejection(&window), Some(WindowRejection::NoClientArea));
    }

    #[test]
    fn a_hidden_window_is_not_a_render_target() {
        let mut window = game_window();
        window.visible = false;
        assert_eq!(rejection(&window), Some(WindowRejection::Invisible));
    }

    #[test]
    fn a_cloaked_window_is_not_a_render_target_even_though_it_is_visible() {
        // The case this guards: a suspended store app leaves a window that is
        // visible, correctly sized, and drawing nothing at all.
        let mut window = game_window();
        window.cloaked = true;
        assert_eq!(rejection(&window), Some(WindowRejection::Cloaked));
    }

    #[test]
    fn a_tool_window_is_not_a_render_target() {
        let mut window = game_window();
        window.tool_window = true;
        assert_eq!(rejection(&window), Some(WindowRejection::ToolWindow));
    }

    #[test]
    fn a_child_window_is_not_a_render_target() {
        let mut window = game_window();
        window.child = true;
        assert_eq!(rejection(&window), Some(WindowRejection::ChildWindow));
    }

    #[test]
    fn an_owned_window_is_a_dialog_and_is_not_a_render_target() {
        let mut window = game_window();
        window.owned = true;
        assert_eq!(rejection(&window), Some(WindowRejection::OwnedWindow));
    }

    #[test]
    fn a_console_window_of_the_same_process_is_not_a_render_target() {
        // A game started from a terminal keeps a console around, at a size that
        // passes every other check.
        let mut window = game_window();
        window.class_name = "ConsoleWindowClass".into();
        assert_eq!(rejection(&window), Some(WindowRejection::ConsoleWindow));
    }

    #[test]
    fn console_class_matching_ignores_case_the_way_win32_does() {
        let mut window = game_window();
        window.class_name = "consolewindowclass".into();
        assert_eq!(rejection(&window), Some(WindowRejection::ConsoleWindow));
    }

    #[test]
    fn a_window_smaller_than_any_game_resolution_is_rejected() {
        let mut window = game_window();
        window.client = Rect::new(0, 0, 300, 120);
        assert_eq!(
            rejection(&window),
            Some(WindowRejection::BelowMinimumSize {
                width: 300,
                height: 120
            })
        );
    }

    #[test]
    fn a_window_at_the_minimum_size_is_accepted() {
        // The boundary is a decision, so it is pinned rather than left to drift
        // with an edit to the comparison.
        let mut window = game_window();
        window.client = Rect::new(0, 0, MINIMUM_SURFACE_WIDTH, MINIMUM_SURFACE_HEIGHT);
        assert_eq!(rejection(&window), None);
    }

    #[test]
    fn a_minimised_window_is_measured_by_its_restore_rectangle() {
        // Win32 parks a minimised window at -32000 with an empty client area.
        // Measuring that would reject the game the moment a player minimises
        // it, and the overlay would forget which window it was attached to.
        let mut window = game_window();
        window.minimized = true;
        window.client = Rect::default();
        window.bounds = Rect::new(-32000, -32000, 160, 28);
        window.restored_bounds = Some(Rect::new(100, 100, 1280, 720));

        assert_eq!(rejection(&window), None);
        assert_eq!(window.drawable_size(), Some((1280, 720)));
    }

    #[test]
    fn a_minimised_window_with_no_restore_rectangle_reports_no_client_area() {
        let mut window = game_window();
        window.minimized = true;
        window.client = Rect::default();
        window.restored_bounds = None;
        assert_eq!(rejection(&window), Some(WindowRejection::NoClientArea));
    }

    #[test]
    fn nothing_is_chosen_from_an_empty_list() {
        assert!(choose_render_window(&[]).is_none());
    }

    #[test]
    fn nothing_is_chosen_when_every_window_is_rejected() {
        let mut splash = game_window();
        splash.client = Rect::new(0, 0, 240, 120);
        let mut console = game_window();
        console.class_name = "ConsoleWindowClass".into();

        assert!(choose_render_window(&[splash, console]).is_none());
    }

    #[test]
    fn the_game_window_wins_over_a_splash_screen_of_the_same_process() {
        let mut splash = game_window();
        splash.hwnd = 0x2000;
        splash.title = "Loading".into();
        splash.caption = false;
        splash.client = Rect::new(0, 0, 480, 320);
        splash.foreground = true;

        let game = game_window();
        let candidates = [splash, game.clone()];
        let chosen = choose_render_window(&candidates).expect("a candidate");
        assert_eq!(chosen.hwnd, game.hwnd);
    }

    #[test]
    fn between_two_windows_of_one_process_the_foreground_one_breaks_a_tie() {
        let mut background = game_window();
        background.hwnd = 0x3000;

        let mut focused = game_window();
        focused.hwnd = 0x4000;
        focused.foreground = true;

        let candidates = [background, focused];
        let chosen = choose_render_window(&candidates).expect("a candidate");
        assert_eq!(chosen.hwnd, 0x4000);
    }

    #[test]
    fn a_console_belonging_to_the_game_never_wins_however_large_it_is() {
        // A maximised terminal is bigger than a windowed game, so size alone
        // would pick the wrong one.
        let mut console = game_window();
        console.hwnd = 0x5000;
        console.class_name = "ConsoleWindowClass".into();
        console.client = Rect::new(0, 0, 2560, 1440);

        let game = game_window();
        let candidates = [console, game.clone()];
        let chosen = choose_render_window(&candidates).expect("a candidate");
        assert_eq!(chosen.hwnd, game.hwnd);
    }

    #[test]
    fn a_minimised_game_still_wins_over_a_smaller_launcher_window() {
        let mut launcher = game_window();
        launcher.hwnd = 0x6000;
        launcher.client = Rect::new(0, 0, 800, 600);
        launcher.foreground = true;

        let mut game = game_window();
        game.minimized = true;
        game.client = Rect::default();
        game.restored_bounds = Some(Rect::new(0, 0, 1920, 1080));

        let candidates = [launcher, game.clone()];
        let chosen = choose_render_window(&candidates).expect("a candidate");
        assert_eq!(chosen.hwnd, game.hwnd);
    }

    #[test]
    fn the_same_list_in_a_different_order_chooses_the_same_window() {
        let mut first = game_window();
        first.hwnd = 0x7000;
        let mut second = game_window();
        second.hwnd = 0x8000;

        let forwards = choose_render_window(&[first.clone(), second.clone()])
            .expect("a candidate")
            .hwnd;
        let backwards = choose_render_window(&[second, first])
            .expect("a candidate")
            .hwnd;
        assert_eq!(forwards, backwards);
    }

    #[test]
    fn a_rectangle_reports_its_own_size() {
        let rect = Rect::new(-10, -20, 100, 50);
        assert_eq!((rect.width(), rect.height()), (100, 50));
        assert_eq!(rect.area(), 5000);
        assert!(!rect.is_empty());
    }

    #[test]
    fn an_inside_out_rectangle_has_no_area_rather_than_a_negative_one() {
        let rect = Rect {
            left: 100,
            top: 100,
            right: 0,
            bottom: 0,
        };
        assert_eq!(rect.area(), 0);
        assert!(rect.is_empty());
    }
}
