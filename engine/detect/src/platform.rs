//! The Win32 side: enumeration, and nothing that makes a decision.
//!
//! Everything here reads facts and hands them to the pure modules. It is the
//! part that cannot be unit tested, so it is kept as thin as it can be made and
//! holds no policy of its own.
//!
//! The rule this module is built around is that a process handle is a detection
//! surface. Anti-cheat drivers see who opens a handle to the game, with what
//! access, and how often, and an overlay that behaves like a debugger is an
//! overlay that gets its user banned. So:
//!
//! - the process id comes from `GetWindowThreadProcessId`, which needs no
//!   handle at all,
//! - process names come from a Toolhelp snapshot, which enumerates every
//!   process in one call instead of opening each one,
//! - loaded modules come from a Toolhelp module snapshot for the same reason.
//!   The alternative, `EnumProcessModulesEx`, needs `PROCESS_QUERY_INFORMATION`
//!   and `PROCESS_VM_READ` on a handle this crate would have to hold, and
//!   `PROCESS_VM_READ` on a game is exactly the access a memory cheat asks for.
//!
//! Nothing in this file calls `OpenProcess`. Where a snapshot fails, the answer
//! is reported as unknown rather than retried with more access.

use std::ffi::c_void;

use windows::Win32::Foundation::{CloseHandle, BOOL, HANDLE, HWND, LPARAM, RECT};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, HMONITOR, MONITORINFO, MONITORINFOEXW,
    MONITOR_DEFAULTTONULL,
};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Module32FirstW, Module32NextW, Process32FirstW, Process32NextW,
    MODULEENTRY32W, PROCESSENTRY32W, TH32CS_SNAPMODULE, TH32CS_SNAPMODULE32, TH32CS_SNAPPROCESS,
};
use windows::Win32::UI::HiDpi::{
    AreDpiAwarenessContextsEqual, GetAwarenessFromDpiAwarenessContext, GetDpiForMonitor,
    GetThreadDpiAwarenessContext, SetProcessDpiAwarenessContext,
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, DPI_AWARENESS_PER_MONITOR_AWARE,
    DPI_AWARENESS_SYSTEM_AWARE, DPI_AWARENESS_UNAWARE, MDT_EFFECTIVE_DPI,
};
use windows::Win32::UI::Shell::{SHQueryUserNotificationState, QUNS_RUNNING_D3D_FULL_SCREEN};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetClientRect, GetForegroundWindow, GetWindow, GetWindowLongPtrW,
    GetWindowPlacement, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
    IsWindowVisible, GWL_EXSTYLE, GWL_STYLE, GW_OWNER, MONITORINFOF_PRIMARY, WINDOWPLACEMENT,
    WINDOW_EX_STYLE, WINDOW_LONG_PTR_INDEX, WINDOW_STYLE, WS_CAPTION, WS_CHILD, WS_EX_TOOLWINDOW,
    WS_THICKFRAME,
};

use crate::display::{DpiAwareness, Monitor};
use crate::graphics::GraphicsProfile;
use crate::presentation::ExclusiveFullscreenHint;
use crate::window::{Rect, WindowFacts};

/// One process, as a Toolhelp snapshot describes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessEntry {
    /// The process id.
    pub pid: u32,
    /// The executable file name with no path, such as `hl2.exe`. Toolhelp does
    /// not give a path without a handle, and the name is enough to match on.
    pub executable: String,
    /// The parent process id, which is how a launcher and the game it started
    /// are related.
    pub parent_pid: u32,
}

/// Ask Windows to treat this process as per-monitor DPI aware, and report what
/// it actually is afterwards.
///
/// Worth calling once at startup, before any window rectangle is read. Under
/// any lesser awareness Windows virtualises the coordinates it hands back, so
/// an overlay placed on them lands in the wrong place on a scaled display.
///
/// Setting it can fail because it was already set, by a manifest or by an
/// earlier call, and that is not an error. Either way the return value is the
/// awareness in force, read back rather than assumed.
pub fn adopt_per_monitor_dpi_awareness() -> DpiAwareness {
    // The result is deliberately ignored. It fails with ACCESS_DENIED when the
    // awareness is already set, which is the normal case for an application
    // that declares it in its manifest, and the read below reports the truth in
    // either case.
    let _ = unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    dpi_awareness()
}

/// The DPI awareness of this process, as Windows sees it.
pub fn dpi_awareness() -> DpiAwareness {
    unsafe {
        let context = GetThreadDpiAwarenessContext();
        if context.0.is_null() {
            return DpiAwareness::Unknown;
        }

        // The awareness enum collapses v1 and v2 into one value, so v2 has to
        // be identified by comparing contexts rather than by reading it.
        if AreDpiAwarenessContextsEqual(context, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)
            .as_bool()
        {
            return DpiAwareness::PerMonitorV2;
        }

        match GetAwarenessFromDpiAwarenessContext(context) {
            DPI_AWARENESS_UNAWARE => DpiAwareness::Unaware,
            DPI_AWARENESS_SYSTEM_AWARE => DpiAwareness::System,
            DPI_AWARENESS_PER_MONITOR_AWARE => DpiAwareness::PerMonitor,
            _ => DpiAwareness::Unknown,
        }
    }
}

/// What the shell says about an exclusive fullscreen Direct3D application, for
/// [`crate::presentation::presentation_mode`].
pub fn exclusive_fullscreen_hint() -> ExclusiveFullscreenHint {
    match unsafe { SHQueryUserNotificationState() } {
        Ok(state) if state == QUNS_RUNNING_D3D_FULL_SCREEN => ExclusiveFullscreenHint::Running,
        Ok(_) => ExclusiveFullscreenHint::NotRunning,
        // The query fails outside an interactive session, and a failure is not
        // evidence of absence. It costs the ability to tell exclusive
        // fullscreen from borderless, so it is worth a line in the log.
        Err(error) => {
            log::debug!("the shell would not say whether a game is fullscreen: {error}");
            ExclusiveFullscreenHint::Unknown
        }
    }
}

/// Every top-level window on the desktop, with the facts the decision needs.
///
/// The enumeration itself only collects handles. Everything else is read
/// afterwards, so no call this crate makes runs inside the enumeration
/// callback, where an unwind would cross a Win32 frame.
pub fn all_windows() -> Vec<WindowFacts> {
    let handles = top_level_handles();
    let foreground = unsafe { GetForegroundWindow() };

    handles
        .into_iter()
        .filter_map(|hwnd| facts_for(hwnd, foreground))
        .collect()
}

/// Every top-level window belonging to one process.
pub fn windows_of_process(pid: u32) -> Vec<WindowFacts> {
    all_windows()
        .into_iter()
        .filter(|window| window.pid == pid)
        .collect()
}

/// Every top-level window whose class matches, case insensitively, the way
/// Win32 compares class names.
pub fn windows_of_class(class_name: &str) -> Vec<WindowFacts> {
    all_windows()
        .into_iter()
        .filter(|window| window.class_name.eq_ignore_ascii_case(class_name))
        .collect()
}

/// The facts about one window, or `None` if the handle is no longer a window.
pub fn window_facts(hwnd: isize) -> Option<WindowFacts> {
    let hwnd = HWND(hwnd as *mut c_void);
    let foreground = unsafe { GetForegroundWindow() };
    facts_for(hwnd, foreground)
}

/// Every running process, from one Toolhelp snapshot.
///
/// An empty list means the snapshot failed, which happens under some sandboxing
/// policies. There is no other way to tell that apart, and a machine with no
/// processes on it is not a case worth modelling.
pub fn processes() -> Vec<ProcessEntry> {
    let snapshot = match unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) } {
        Ok(snapshot) => snapshot,
        Err(error) => {
            // Worth a warning rather than a debug line: with no process list,
            // every executable target silently finds nothing, and the user sees
            // an app that never notices their game.
            log::warn!("could not snapshot the process list: {error}");
            return Vec::new();
        }
    };
    let snapshot = OwnedHandle(snapshot);

    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut found = Vec::new();

    unsafe {
        if Process32FirstW(snapshot.0, &mut entry).is_err() {
            return found;
        }
        loop {
            found.push(ProcessEntry {
                pid: entry.th32ProcessID,
                executable: from_wide(&entry.szExeFile),
                parent_pid: entry.th32ParentProcessID,
            });
            if Process32NextW(snapshot.0, &mut entry).is_err() {
                break;
            }
        }
    }

    found
}

/// Every process whose executable file name matches, case insensitively.
pub fn processes_named(executable: &str) -> Vec<ProcessEntry> {
    processes()
        .into_iter()
        .filter(|process| process.executable.eq_ignore_ascii_case(executable))
        .collect()
}

/// The executable file name of one process, or `None` when it is gone or the
/// snapshot could not be taken.
pub fn process_name(pid: u32) -> Option<String> {
    processes()
        .into_iter()
        .find(|process| process.pid == pid)
        .map(|process| process.executable)
}

/// Which graphics runtimes a process has loaded.
///
/// Returns [`GraphicsProfile::unreadable`] when the module list cannot be read,
/// which is a normal outcome rather than a failure: an elevated or protected
/// process is not enumerable from here, and this crate will not open a handle
/// to make it so.
pub fn graphics_profile(pid: u32) -> GraphicsProfile {
    match module_names(pid) {
        Some(modules) => GraphicsProfile::from_modules(modules),
        None => GraphicsProfile::unreadable(),
    }
}

/// The base names of every module loaded in a process, or `None` when the
/// snapshot could not be taken.
fn module_names(pid: u32) -> Option<Vec<String>> {
    // A module snapshot of a process that is still loading fails with
    // ERROR_BAD_LENGTH, and the documented answer is to take it again. Three
    // attempts, because a process that is still moving after that will be
    // caught by the next scan anyway.
    let mut snapshot = None;
    for _ in 0..3 {
        // SNAPMODULE32 is what makes this work against a 32-bit game from a
        // 64-bit host, which is most of the older catalogue.
        match unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, pid) } {
            Ok(handle) => {
                snapshot = Some(OwnedHandle(handle));
                break;
            }
            Err(error) => {
                log::debug!("module snapshot of pid {pid} failed: {error}");
                continue;
            }
        }
    }
    let snapshot = snapshot?;

    let mut entry = MODULEENTRY32W {
        dwSize: std::mem::size_of::<MODULEENTRY32W>() as u32,
        ..Default::default()
    };
    let mut modules = Vec::new();

    unsafe {
        if Module32FirstW(snapshot.0, &mut entry).is_err() {
            return None;
        }
        loop {
            modules.push(from_wide(&entry.szModule));
            if Module32NextW(snapshot.0, &mut entry).is_err() {
                break;
            }
        }
    }

    Some(modules)
}

/// A snapshot handle that closes itself.
///
/// This is a handle to the snapshot, not to any process, and it is the only
/// handle this crate holds.
struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.0) };
    }
}

fn top_level_handles() -> Vec<HWND> {
    let mut handles: Vec<HWND> = Vec::new();
    let _ = unsafe {
        EnumWindows(
            Some(collect_handle),
            LPARAM(std::ptr::addr_of_mut!(handles) as isize),
        )
    };
    handles
}

unsafe extern "system" fn collect_handle(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let handles = &mut *(lparam.0 as *mut Vec<HWND>);
    handles.push(hwnd);
    // Keep going. Stopping early would make the window list depend on Z order.
    BOOL(1)
}

fn facts_for(hwnd: HWND, foreground: HWND) -> Option<WindowFacts> {
    let mut pid = 0u32;
    let thread = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    // A window that has been destroyed since the enumeration reports no thread.
    if thread == 0 || pid == 0 {
        return None;
    }

    let style = WINDOW_STYLE(window_long(hwnd, GWL_STYLE) as u32);
    let ex_style = WINDOW_EX_STYLE(window_long(hwnd, GWL_EXSTYLE) as u32);

    Some(WindowFacts {
        hwnd: hwnd.0 as isize,
        pid,
        class_name: class_name(hwnd),
        title: title(hwnd),
        bounds: window_rect(hwnd),
        client: client_rect(hwnd),
        restored_bounds: restored_bounds(hwnd),
        visible: unsafe { IsWindowVisible(hwnd) }.as_bool(),
        minimized: unsafe { IsIconic(hwnd) }.as_bool(),
        cloaked: is_cloaked(hwnd),
        foreground: hwnd == foreground,
        child: style.contains(WS_CHILD),
        owned: unsafe { GetWindow(hwnd, GW_OWNER) }.is_ok_and(|owner| !owner.0.is_null()),
        tool_window: ex_style.contains(WS_EX_TOOLWINDOW),
        caption: style.contains(WS_CAPTION),
        resizable: style.contains(WS_THICKFRAME),
        monitor: monitor_of(hwnd),
    })
}

#[cfg(target_pointer_width = "64")]
fn window_long(hwnd: HWND, index: WINDOW_LONG_PTR_INDEX) -> isize {
    unsafe { GetWindowLongPtrW(hwnd, index) }
}

#[cfg(not(target_pointer_width = "64"))]
fn window_long(hwnd: HWND, index: WINDOW_LONG_PTR_INDEX) -> isize {
    // The pointer sized form does not exist on 32-bit Windows, where a long
    // already is pointer sized.
    unsafe { windows::Win32::UI::WindowsAndMessaging::GetWindowLongW(hwnd, index) as isize }
}

fn class_name(hwnd: HWND) -> String {
    // 256 is the documented limit on a registered class name.
    let mut buffer = [0u16; 256];
    let written = unsafe { GetClassNameW(hwnd, &mut buffer) };
    if written <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buffer[..written as usize])
}

fn title(hwnd: HWND) -> String {
    // Titles are not length limited, and a truncated one is only ever shown to
    // a human, so a fixed buffer is the right trade against a call per window
    // to measure first.
    let mut buffer = [0u16; 512];
    let written = unsafe { GetWindowTextW(hwnd, &mut buffer) };
    if written <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buffer[..written as usize])
}

fn window_rect(hwnd: HWND) -> Rect {
    let mut rect = RECT::default();
    match unsafe { GetWindowRect(hwnd, &mut rect) } {
        Ok(()) => rect.into(),
        Err(_) => Rect::default(),
    }
}

fn client_rect(hwnd: HWND) -> Rect {
    let mut rect = RECT::default();
    match unsafe { GetClientRect(hwnd, &mut rect) } {
        Ok(()) => rect.into(),
        Err(_) => Rect::default(),
    }
}

/// Where the window goes when it is restored.
///
/// This is the only size a minimised window has. The rectangle is in workspace
/// coordinates rather than screen ones, so its position can be off by the task
/// bar, and only its size is used.
fn restored_bounds(hwnd: HWND) -> Option<Rect> {
    let mut placement = WINDOWPLACEMENT {
        length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
        ..Default::default()
    };
    match unsafe { GetWindowPlacement(hwnd, &mut placement) } {
        Ok(()) => Some(placement.rcNormalPosition.into()),
        Err(_) => None,
    }
}

fn is_cloaked(hwnd: HWND) -> bool {
    let mut cloaked = 0u32;
    let result = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            std::ptr::addr_of_mut!(cloaked).cast::<c_void>(),
            std::mem::size_of::<u32>() as u32,
        )
    };
    // A failure here means the DWM has nothing to say about this window, which
    // is not the same as the window being cloaked.
    result.is_ok() && cloaked != 0
}

fn monitor_of(hwnd: HWND) -> Option<Monitor> {
    let handle = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONULL) };
    if handle.0.is_null() {
        return None;
    }
    monitor_facts(handle)
}

/// Read a monitor, or report it absent.
///
/// A monitor whose DPI cannot be read comes back as `None` rather than as a
/// monitor at 96. The default DPI is a real value meaning no scaling, and
/// handing it back for an unreadable one would be a guess that reads as a fact.
fn monitor_facts(handle: HMONITOR) -> Option<Monitor> {
    let mut info = MONITORINFOEXW {
        monitorInfo: MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFOEXW>() as u32,
            ..Default::default()
        },
        ..Default::default()
    };

    let read =
        unsafe { GetMonitorInfoW(handle, std::ptr::addr_of_mut!(info).cast::<MONITORINFO>()) };
    if !read.as_bool() {
        return None;
    }

    let mut dpi_x = 0u32;
    let mut dpi_y = 0u32;
    unsafe { GetDpiForMonitor(handle, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) }.ok()?;

    Some(Monitor {
        handle: handle.0 as isize,
        device_name: from_wide(&info.szDevice),
        bounds: info.monitorInfo.rcMonitor.into(),
        work_area: info.monitorInfo.rcWork.into(),
        // Windows reports the same value on both axes for every display it
        // supports, and the overlay has one scale factor to draw at.
        dpi: dpi_x,
        primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
    })
}

impl From<RECT> for Rect {
    fn from(rect: RECT) -> Self {
        Rect {
            left: rect.left,
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
        }
    }
}

/// A fixed size wide buffer as a string, cut at the first nul.
fn from_wide(buffer: &[u16]) -> String {
    let end = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    // These run against the live desktop, so they assert the shape of what
    // comes back rather than any particular window, which is all that can be
    // true on every machine.

    #[test]
    fn a_fixed_buffer_is_cut_at_the_first_nul() {
        let mut buffer = [0u16; 8];
        for (slot, character) in buffer.iter_mut().zip("d3d11".encode_utf16()) {
            *slot = character;
        }
        assert_eq!(from_wide(&buffer), "d3d11");
    }

    #[test]
    fn a_full_buffer_with_no_nul_is_read_to_the_end() {
        let buffer: Vec<u16> = "d3d11.dll".encode_utf16().collect();
        assert_eq!(from_wide(&buffer), "d3d11.dll");
    }

    #[test]
    fn the_desktop_has_windows_and_every_one_of_them_has_a_process() {
        let windows = all_windows();
        assert!(!windows.is_empty(), "no top level windows at all");
        assert!(windows.iter().all(|window| window.pid != 0));
    }

    #[test]
    fn the_test_runner_finds_itself_in_the_process_list() {
        // Uses this process, which is the one target this crate is allowed to
        // touch, and proves the snapshot walk reaches the end of the list
        // rather than stopping at the first entry.
        let pid = std::process::id();
        let name = process_name(pid).expect("this process is in the snapshot");
        assert!(
            name.to_ascii_lowercase().ends_with(".exe"),
            "unexpected executable name {name}"
        );
    }

    #[test]
    fn the_test_runner_can_read_its_own_module_list() {
        let profile = graphics_profile(std::process::id());
        assert!(profile.is_readable());
    }

    #[test]
    fn a_process_id_that_does_not_exist_has_no_name_rather_than_a_wrong_one() {
        // Process ids are multiples of four and this one is not, so it can
        // never be assigned.
        assert_eq!(process_name(u32::MAX - 1), None);
    }

    #[test]
    fn an_unreadable_module_list_is_reported_as_unknown_rather_than_as_empty() {
        // A process id that cannot be assigned, standing in for the protected
        // process this will really meet. The distinction being checked is the
        // one that matters: nothing readable is not the same as nothing
        // loaded, and only one of the two may be shown as unsupported.
        let profile = graphics_profile(u32::MAX - 1);

        assert!(!profile.is_readable());
        assert_eq!(
            profile.support(),
            crate::graphics::RendererSupport::Unreadable
        );
    }

    #[test]
    fn a_window_handle_that_is_not_a_window_yields_no_facts() {
        assert_eq!(window_facts(0), None);
    }

    #[test]
    fn the_dpi_awareness_of_this_process_can_be_read() {
        // Whatever it is, it must not be reported as unknown: an unknown
        // awareness would mean the coordinates elsewhere cannot be trusted.
        assert_ne!(dpi_awareness(), DpiAwareness::Unknown);
    }
}
