//! Which display the game is on, what its DPI is, and what changed when the
//! window moved.
//!
//! Two monitors that disagree about DPI are the normal case on a laptop with an
//! external screen, and dragging a game between them changes the size of a
//! physical pixel underneath the overlay without changing anything the game
//! reports. So the DPI is carried alongside the geometry everywhere, and the
//! comparison that says whether the overlay has to be rebuilt is a pure
//! function with tests.

use crate::window::Rect;

/// The DPI Windows treats as a scale factor of 1.0. Named because `96` on its
/// own in an expression is not obviously this.
pub const DEFAULT_DPI: u32 = 96;

/// One display, as the detector sees it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Monitor {
    /// The raw `HMONITOR` value. `isize` rather than the `windows` type so this
    /// struct does not pin consumers to one version of that crate. A monitor
    /// handle is only valid until the display topology changes, so treat it as
    /// an identity for comparison rather than something to keep.
    pub handle: isize,
    /// The device name, such as `\\.\DISPLAY1`. Empty when it could not be
    /// read.
    pub device_name: String,
    /// The full monitor rectangle in virtual desktop coordinates.
    pub bounds: Rect,
    /// The part of the monitor not covered by the task bar and other appbars.
    pub work_area: Rect,
    /// Effective dots per inch, from `GetDpiForMonitor`, which reports the
    /// monitor's real DPI rather than the value the calling process is allowed
    /// to see.
    pub dpi: u32,
    /// True for the display the shell treats as primary.
    pub primary: bool,
}

impl Monitor {
    /// A monitor with a given rectangle and DPI, work area equal to bounds.
    ///
    /// The full struct is tedious to build by hand and most tests care about
    /// two fields, so this exists to keep the interesting values visible in the
    /// test that sets them.
    pub fn new(bounds: Rect, dpi: u32) -> Self {
        Monitor {
            handle: 0,
            device_name: String::new(),
            bounds,
            work_area: bounds,
            dpi,
            primary: false,
        }
    }

    /// The scale factor the overlay has to draw at, where 1.0 is 96 DPI.
    ///
    /// 150 percent scaling reports 144, and this returns 1.5.
    pub fn scale(&self) -> f32 {
        self.dpi as f32 / DEFAULT_DPI as f32
    }
}

/// How the calling process understands DPI.
///
/// This matters to a consumer of this crate rather than to this crate itself.
/// Window rectangles read from a process that is not per-monitor aware are
/// virtualised by Windows, so they can disagree with the pixels on the screen.
/// The detector reports what it is running under so a caller can tell whether
/// the coordinates it is being handed are physical.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DpiAwareness {
    /// Everything is scaled by the system and reported as if it were 96 DPI.
    Unaware,
    /// One scale factor for the whole session, fixed at login.
    System,
    /// Per monitor, first generation. Window rectangles are physical, but
    /// non-client area is not rescaled.
    PerMonitor,
    /// Per monitor, second generation. What this crate asks for.
    PerMonitorV2,
    /// The awareness could not be read, which is not the same as unaware.
    Unknown,
}

impl std::fmt::Display for DpiAwareness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DpiAwareness::Unaware => write!(f, "DPI unaware"),
            DpiAwareness::System => write!(f, "system DPI aware"),
            DpiAwareness::PerMonitor => write!(f, "per monitor DPI aware"),
            DpiAwareness::PerMonitorV2 => write!(f, "per monitor DPI aware, v2"),
            DpiAwareness::Unknown => write!(f, "DPI awareness unknown"),
        }
    }
}

impl DpiAwareness {
    /// True when window rectangles from this process are real screen pixels.
    ///
    /// Under the other awareness levels Windows lies to the process about
    /// coordinates, and a rectangle the overlay is placed at will not line up
    /// with the game.
    pub fn reports_physical_pixels(&self) -> bool {
        matches!(self, DpiAwareness::PerMonitor | DpiAwareness::PerMonitorV2)
    }
}

/// What happened to the display under the game window between two observations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayChange {
    /// Same monitor, same DPI.
    Unchanged,
    /// Still on the same monitor, but its DPI changed underneath the window.
    /// A user changing the scaling slider does this without moving anything.
    DpiChanged {
        /// The DPI before.
        from: u32,
        /// The DPI now.
        to: u32,
    },
    /// The window is on a different monitor now.
    MovedToMonitor {
        /// True when the new monitor scales differently from the old one, which
        /// is the case that costs the overlay a rebuild rather than a redraw.
        dpi_changed: bool,
    },
    /// The window was off every display and is now on one, which is what a
    /// restore from minimised looks like.
    Acquired,
    /// The window is no longer on any display. Minimising does this, and so
    /// does unplugging the monitor it was on.
    Lost,
}

impl DisplayChange {
    /// True when the overlay has to rebuild its surface rather than carry on
    /// drawing.
    ///
    /// A move between two monitors at the same DPI is only a position change,
    /// so it is deliberately not on this list. Anything that changes the size
    /// of a physical pixel is.
    pub fn requires_rebuild(&self) -> bool {
        match self {
            DisplayChange::Unchanged => false,
            DisplayChange::DpiChanged { .. } => true,
            DisplayChange::MovedToMonitor { dpi_changed } => *dpi_changed,
            // Acquiring a display means there was nothing to draw on before,
            // so whatever surface exists was built for a monitor that is gone.
            DisplayChange::Acquired => true,
            DisplayChange::Lost => false,
        }
    }
}

/// Compare the display under a window between two observations.
///
/// Monitors are compared by handle, not by rectangle, because two displays can
/// share a rectangle while one is being reconfigured, and because a single
/// monitor keeps its handle when the user drags it around in display settings.
pub fn display_change(before: Option<&Monitor>, after: Option<&Monitor>) -> DisplayChange {
    match (before, after) {
        (None, None) => DisplayChange::Unchanged,
        (None, Some(_)) => DisplayChange::Acquired,
        (Some(_), None) => DisplayChange::Lost,
        (Some(old), Some(new)) => {
            if old.handle == new.handle {
                if old.dpi == new.dpi {
                    DisplayChange::Unchanged
                } else {
                    DisplayChange::DpiChanged {
                        from: old.dpi,
                        to: new.dpi,
                    }
                }
            } else {
                DisplayChange::MovedToMonitor {
                    dpi_changed: old.dpi != new.dpi,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(handle: isize, dpi: u32) -> Monitor {
        Monitor {
            handle,
            dpi,
            ..Monitor::new(Rect::new(0, 0, 1920, 1080), dpi)
        }
    }

    #[test]
    fn ninety_six_dots_per_inch_is_a_scale_of_one() {
        assert_eq!(Monitor::new(Rect::default(), 96).scale(), 1.0);
    }

    #[test]
    fn a_hundred_and_fifty_percent_scaling_reports_one_and_a_half() {
        assert_eq!(Monitor::new(Rect::default(), 144).scale(), 1.5);
    }

    #[test]
    fn staying_on_one_monitor_at_one_scale_is_no_change() {
        let before = monitor(1, 96);
        let after = monitor(1, 96);
        assert_eq!(
            display_change(Some(&before), Some(&after)),
            DisplayChange::Unchanged
        );
        assert!(!display_change(Some(&before), Some(&after)).requires_rebuild());
    }

    #[test]
    fn changing_the_scaling_slider_is_a_dpi_change_without_a_move() {
        let before = monitor(1, 96);
        let after = monitor(1, 144);
        let change = display_change(Some(&before), Some(&after));

        assert_eq!(change, DisplayChange::DpiChanged { from: 96, to: 144 });
        assert!(change.requires_rebuild());
    }

    #[test]
    fn dragging_between_two_monitors_of_the_same_dpi_does_not_need_a_rebuild() {
        let before = monitor(1, 96);
        let after = monitor(2, 96);
        let change = display_change(Some(&before), Some(&after));

        assert_eq!(change, DisplayChange::MovedToMonitor { dpi_changed: false });
        assert!(!change.requires_rebuild());
    }

    #[test]
    fn dragging_to_a_monitor_that_scales_differently_needs_a_rebuild() {
        // The laptop and external screen case, which is the whole reason this
        // comparison exists.
        let laptop = monitor(1, 192);
        let external = monitor(2, 96);
        let change = display_change(Some(&laptop), Some(&external));

        assert_eq!(change, DisplayChange::MovedToMonitor { dpi_changed: true });
        assert!(change.requires_rebuild());
    }

    #[test]
    fn two_monitors_sharing_a_rectangle_are_still_two_monitors() {
        // Duplicated displays have the same bounds and different handles.
        let first = monitor(1, 96);
        let second = monitor(2, 96);
        assert_eq!(first.bounds, second.bounds);
        assert_eq!(
            display_change(Some(&first), Some(&second)),
            DisplayChange::MovedToMonitor { dpi_changed: false }
        );
    }

    #[test]
    fn a_window_that_leaves_every_display_reports_the_loss() {
        let before = monitor(1, 96);
        assert_eq!(display_change(Some(&before), None), DisplayChange::Lost);
        assert!(!display_change(Some(&before), None).requires_rebuild());
    }

    #[test]
    fn a_window_that_arrives_on_a_display_needs_a_surface_built_for_it() {
        let after = monitor(1, 96);
        let change = display_change(None, Some(&after));
        assert_eq!(change, DisplayChange::Acquired);
        assert!(change.requires_rebuild());
    }

    #[test]
    fn a_window_that_was_never_on_a_display_reports_no_change() {
        assert_eq!(display_change(None, None), DisplayChange::Unchanged);
    }

    #[test]
    fn only_per_monitor_awareness_gives_physical_pixels() {
        assert!(DpiAwareness::PerMonitorV2.reports_physical_pixels());
        assert!(DpiAwareness::PerMonitor.reports_physical_pixels());
        assert!(!DpiAwareness::System.reports_physical_pixels());
        assert!(!DpiAwareness::Unaware.reports_physical_pixels());
    }

    #[test]
    fn unknown_awareness_is_not_treated_as_aware() {
        // An unreadable awareness is absent, and absent must not be optimistic:
        // placing an overlay on coordinates that might be virtualised is the
        // failure this guards.
        assert!(!DpiAwareness::Unknown.reports_physical_pixels());
    }
}
