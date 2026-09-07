//! Which graphics API a process is actually using, decided from the modules it
//! has loaded.
//!
//! A game on an API this engine does not hook has to be told apart from a game
//! the engine failed on. An overlay that silently never appears is the worst
//! version of that, because the user cannot tell it from a bug, so the API is
//! named even when the answer is that it is not supported.
//!
//! The evidence is the loaded module list, which is a good signal and not a
//! perfect one: a process can hold `d3d11.dll` open without presenting through
//! it. So both the raw list and the conclusion drawn from it are public, and
//! the conclusion says how confident it is.

/// A graphics API the engine can recognise.
///
/// The list is what a Windows game realistically presents through. Anything not
/// on it comes back as no API rather than a wrong one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphicsApi {
    /// Direct3D 9, including the 9on12 shim.
    Direct3D9,
    /// Direct3D 10 and 10.1.
    Direct3D10,
    /// Direct3D 11. The first API this engine supports.
    Direct3D11,
    /// Direct3D 12.
    Direct3D12,
    /// Vulkan, through the loader.
    Vulkan,
    /// OpenGL, through the Windows ICD.
    OpenGl,
}

impl std::fmt::Display for GraphicsApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraphicsApi::Direct3D9 => write!(f, "Direct3D 9"),
            GraphicsApi::Direct3D10 => write!(f, "Direct3D 10"),
            GraphicsApi::Direct3D11 => write!(f, "Direct3D 11"),
            GraphicsApi::Direct3D12 => write!(f, "Direct3D 12"),
            GraphicsApi::Vulkan => write!(f, "Vulkan"),
            GraphicsApi::OpenGl => write!(f, "OpenGL"),
        }
    }
}

impl GraphicsApi {
    /// True when the engine can draw over this API today.
    ///
    /// One API that works beats five that half work, so this is deliberately a
    /// single name and the rest are reported by name as unsupported.
    pub fn is_supported(&self) -> bool {
        matches!(self, GraphicsApi::Direct3D11)
    }
}

/// The modules that identify each API, matched on the file name only, case
/// insensitively, because a module path is not stable and Win32 file names are
/// not case sensitive.
///
/// `d3d9on12.dll` and `d3d11on12.dll` are the compatibility shims. A process
/// using the 9on12 shim is still writing Direct3D 9 calls, so it is listed as
/// Direct3D 9. `d3d11on12.dll` is deliberately absent: it means a Direct3D 12
/// title is running some Direct3D 11 code on its own device, and `d3d12.dll`
/// will be loaded alongside it to say so.
const MODULE_TABLE: [(&str, GraphicsApi); 11] = [
    ("d3d9.dll", GraphicsApi::Direct3D9),
    ("d3d9on12.dll", GraphicsApi::Direct3D9),
    ("d3d10.dll", GraphicsApi::Direct3D10),
    ("d3d10_1.dll", GraphicsApi::Direct3D10),
    ("d3d10core.dll", GraphicsApi::Direct3D10),
    ("d3d11.dll", GraphicsApi::Direct3D11),
    ("d3d12.dll", GraphicsApi::Direct3D12),
    ("d3d12core.dll", GraphicsApi::Direct3D12),
    ("vulkan-1.dll", GraphicsApi::Vulkan),
    ("opengl32.dll", GraphicsApi::OpenGl),
    ("libglesv2.dll", GraphicsApi::OpenGl),
];

/// Precedence when a process has loaded more than one graphics runtime, most
/// telling first.
///
/// Multiple runtimes in one process is normal rather than exceptional. A
/// Direct3D 12 title loads `d3d11.dll` for D3D11On12 interop, and a game with a
/// backend menu can have loaded the one the user is not using. The order is
/// argued rather than arbitrary: a Direct3D 11 title has no reason to load
/// `d3d12.dll`, while the reverse happens by design, and the same argument puts
/// Vulkan above both. OpenGL is last because `opengl32.dll` gets pulled in by
/// unrelated Windows components and is the weakest evidence on the list.
const PRECEDENCE: [GraphicsApi; 6] = [
    GraphicsApi::Vulkan,
    GraphicsApi::Direct3D12,
    GraphicsApi::Direct3D11,
    GraphicsApi::Direct3D10,
    GraphicsApi::Direct3D9,
    GraphicsApi::OpenGl,
];

/// The API a single module name identifies, if any.
///
/// Accepts a full path or a bare file name.
pub fn api_for_module(module: &str) -> Option<GraphicsApi> {
    let file_name = module
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(module)
        .trim_end_matches('\0');

    MODULE_TABLE
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(file_name))
        .map(|(_, api)| *api)
}

/// What the engine can do about the renderer a process is using.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RendererSupport {
    /// An API the engine draws over, named so the app can say which.
    Supported(GraphicsApi),
    /// A recognised API the engine does not draw over yet. The name is the
    /// point: the app can say "this game uses Vulkan, which is not supported
    /// yet" rather than showing nothing.
    Unsupported(GraphicsApi),
    /// The module list was read and held no graphics runtime. A game that has
    /// started but not yet created a device looks like this, and so does a
    /// launcher process that never will.
    NoGraphicsModuleLoaded,
    /// The module list could not be read at all, so nothing is known. Not the
    /// same as a process with no graphics module, and not to be reported as
    /// unsupported.
    Unreadable,
}

impl std::fmt::Display for RendererSupport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RendererSupport::Supported(api) => write!(f, "{api}, supported"),
            RendererSupport::Unsupported(api) => write!(f, "{api}, not supported yet"),
            RendererSupport::NoGraphicsModuleLoaded => {
                write!(f, "no graphics runtime loaded")
            }
            RendererSupport::Unreadable => write!(f, "graphics runtime unknown"),
        }
    }
}

/// Everything the detector learned about a process's graphics runtimes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphicsProfile {
    loaded: Vec<GraphicsApi>,
    readable: bool,
}

impl GraphicsProfile {
    /// Build a profile from the module names a process has loaded.
    ///
    /// Names may be paths or bare file names, in any order, with duplicates.
    pub fn from_modules<I, S>(modules: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut loaded: Vec<GraphicsApi> = Vec::new();
        for module in modules {
            if let Some(api) = api_for_module(module.as_ref()) {
                if !loaded.contains(&api) {
                    loaded.push(api);
                }
            }
        }
        loaded.sort_by_key(|api| {
            PRECEDENCE
                .iter()
                .position(|candidate| candidate == api)
                .unwrap_or(PRECEDENCE.len())
        });

        GraphicsProfile {
            loaded,
            readable: true,
        }
    }

    /// A profile for a process whose module list could not be read.
    ///
    /// This is a real outcome rather than an error path. Reading the list needs
    /// the process to be enumerable, and an elevated or protected one is not,
    /// so the honest answer is that nothing is known.
    pub fn unreadable() -> Self {
        GraphicsProfile {
            loaded: Vec::new(),
            readable: false,
        }
    }

    /// Every recognised graphics runtime the process has loaded, in precedence
    /// order. Empty when the list was read and held none, and also empty when
    /// it could not be read, which [`GraphicsProfile::is_readable`] separates.
    pub fn loaded(&self) -> &[GraphicsApi] {
        &self.loaded
    }

    /// False when the module list could not be read.
    pub fn is_readable(&self) -> bool {
        self.readable
    }

    /// The API the process is most likely presenting through, or `None` when
    /// there is no evidence either way.
    ///
    /// This is an inference from precedence, not a certainty. The evidence it
    /// was drawn from stays available in [`GraphicsProfile::loaded`].
    pub fn primary(&self) -> Option<GraphicsApi> {
        self.loaded.first().copied()
    }

    /// What the engine can do about this process.
    pub fn support(&self) -> RendererSupport {
        if !self.readable {
            return RendererSupport::Unreadable;
        }
        match self.primary() {
            Some(api) if api.is_supported() => RendererSupport::Supported(api),
            Some(api) => RendererSupport::Unsupported(api),
            None => RendererSupport::NoGraphicsModuleLoaded,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_module_name_is_matched_case_insensitively() {
        assert_eq!(api_for_module("D3D11.DLL"), Some(GraphicsApi::Direct3D11));
        assert_eq!(api_for_module("d3d11.dll"), Some(GraphicsApi::Direct3D11));
    }

    #[test]
    fn a_full_path_is_matched_on_its_file_name() {
        assert_eq!(
            api_for_module(r"C:\Windows\System32\vulkan-1.dll"),
            Some(GraphicsApi::Vulkan)
        );
        assert_eq!(
            api_for_module("C:/Windows/System32/opengl32.dll"),
            Some(GraphicsApi::OpenGl)
        );
    }

    #[test]
    fn a_module_that_identifies_nothing_yields_no_api() {
        // dxgi.dll is present for every Direct3D version and half the shell, so
        // it says nothing about which API a game is using.
        assert_eq!(api_for_module("dxgi.dll"), None);
        assert_eq!(api_for_module("kernel32.dll"), None);
        assert_eq!(api_for_module(""), None);
    }

    #[test]
    fn the_direct3d_9_shim_still_reports_direct3d_9() {
        assert_eq!(api_for_module("d3d9on12.dll"), Some(GraphicsApi::Direct3D9));
    }

    #[test]
    fn a_process_with_direct3d_11_is_supported() {
        let profile = GraphicsProfile::from_modules(["kernel32.dll", "dxgi.dll", "d3d11.dll"]);

        assert_eq!(profile.primary(), Some(GraphicsApi::Direct3D11));
        assert_eq!(
            profile.support(),
            RendererSupport::Supported(GraphicsApi::Direct3D11)
        );
    }

    #[test]
    fn a_vulkan_process_is_named_rather_than_reported_as_nothing() {
        let profile = GraphicsProfile::from_modules(["vulkan-1.dll"]);

        assert_eq!(
            profile.support(),
            RendererSupport::Unsupported(GraphicsApi::Vulkan)
        );
        assert_eq!(profile.support().to_string(), "Vulkan, not supported yet");
    }

    #[test]
    fn every_unsupported_api_reports_its_own_name() {
        for (module, api) in [
            ("d3d12.dll", GraphicsApi::Direct3D12),
            ("vulkan-1.dll", GraphicsApi::Vulkan),
            ("opengl32.dll", GraphicsApi::OpenGl),
            ("d3d9.dll", GraphicsApi::Direct3D9),
            ("d3d10.dll", GraphicsApi::Direct3D10),
        ] {
            let profile = GraphicsProfile::from_modules([module]);
            assert_eq!(profile.support(), RendererSupport::Unsupported(api));
        }
    }

    #[test]
    fn direct3d_12_wins_over_the_direct3d_11_it_loads_for_interop() {
        // D3D11On12 puts d3d11.dll in a Direct3D 12 game. Reading that as a
        // supported title would attach a D3D11 hook to a D3D12 swapchain.
        let profile = GraphicsProfile::from_modules(["d3d11.dll", "d3d12.dll", "dxgi.dll"]);

        assert_eq!(profile.primary(), Some(GraphicsApi::Direct3D12));
        assert_eq!(
            profile.support(),
            RendererSupport::Unsupported(GraphicsApi::Direct3D12)
        );
    }

    #[test]
    fn vulkan_wins_over_a_direct3d_runtime_loaded_alongside_it() {
        let profile = GraphicsProfile::from_modules(["d3d11.dll", "vulkan-1.dll"]);
        assert_eq!(profile.primary(), Some(GraphicsApi::Vulkan));
    }

    #[test]
    fn opengl_loses_to_anything_else_because_it_is_the_weakest_evidence() {
        let profile = GraphicsProfile::from_modules(["opengl32.dll", "d3d9.dll"]);
        assert_eq!(profile.primary(), Some(GraphicsApi::Direct3D9));
    }

    #[test]
    fn the_evidence_stays_available_next_to_the_conclusion() {
        // The app should be able to say what it saw, not only what it decided.
        let profile = GraphicsProfile::from_modules(["d3d11.dll", "d3d12.dll", "opengl32.dll"]);

        assert_eq!(
            profile.loaded(),
            [
                GraphicsApi::Direct3D12,
                GraphicsApi::Direct3D11,
                GraphicsApi::OpenGl
            ]
        );
    }

    #[test]
    fn a_module_listed_twice_is_reported_once() {
        let profile = GraphicsProfile::from_modules([
            r"C:\Windows\System32\d3d11.dll",
            r"C:\Windows\SysWOW64\d3d11.dll",
        ]);

        assert_eq!(profile.loaded(), [GraphicsApi::Direct3D11]);
    }

    #[test]
    fn a_process_with_no_graphics_module_is_not_reported_as_unsupported() {
        // A game that has started but not yet created a device looks like this.
        // Calling it unsupported would put a permanent wrong answer on screen a
        // second before the right one was available.
        let profile = GraphicsProfile::from_modules(["kernel32.dll", "user32.dll"]);

        assert_eq!(profile.primary(), None);
        assert_eq!(profile.support(), RendererSupport::NoGraphicsModuleLoaded);
    }

    #[test]
    fn a_module_list_that_could_not_be_read_is_absent_not_empty() {
        let profile = GraphicsProfile::unreadable();

        assert!(!profile.is_readable());
        assert_eq!(profile.support(), RendererSupport::Unreadable);
        assert_ne!(
            profile.support(),
            GraphicsProfile::from_modules::<[&str; 0], &str>([]).support()
        );
    }

    #[test]
    fn only_direct3d_11_is_supported_today() {
        assert!(GraphicsApi::Direct3D11.is_supported());
        for api in [
            GraphicsApi::Direct3D9,
            GraphicsApi::Direct3D10,
            GraphicsApi::Direct3D12,
            GraphicsApi::Vulkan,
            GraphicsApi::OpenGl,
        ] {
            assert!(!api.is_supported(), "{api} should not be supported yet");
        }
    }
}
