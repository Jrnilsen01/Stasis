//! A D3D11 window that presents frames, standing in for a game.
//!
//! This exists because the overlay engine cannot be tested against a real game.
//! Attaching an unknown, unsigned DLL to an anti-cheat protected process is the
//! exact thing that gets a user banned, and the project cannot ask contributors
//! to risk their own accounts to run its test suite. So the harness presents
//! frames the same way a game does, through `IDXGISwapChain::Present`, and the
//! hook is developed and verified against this instead.
//!
//! It is deliberately the least interesting D3D11 application that is still
//! representative: a real swapchain, a real present loop, a real render target,
//! and a background colour that changes every frame so a frozen present is
//! obvious to the eye rather than only in a log.
//!
//! Run it with `cargo run -p stasis-harness`. It prints its own process id and
//! swapchain pointer, which is what the injector and the hook need.

use std::ffi::c_void;

use windows::core::{w, Interface, Result, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDeviceAndSwapChain, ID3D11Device, ID3D11DeviceContext, ID3D11RenderTargetView,
    ID3D11Texture2D, D3D11_SDK_VERSION,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_MODE_DESC, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    IDXGISwapChain, DXGI_PRESENT, DXGI_SWAP_CHAIN_DESC, DXGI_SWAP_EFFECT_DISCARD,
    DXGI_USAGE_RENDER_TARGET_OUTPUT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, PeekMessageW, PostQuitMessage,
    RegisterClassExW, ShowWindow, TranslateMessage, CS_HREDRAW, CS_VREDRAW, MSG, PM_REMOVE,
    SW_SHOW, WM_DESTROY, WNDCLASSEXW, WS_OVERLAPPEDWINDOW,
};

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;

/// The window class name doubles as the marker the detector matches on, so the
/// detection tests have something deterministic to find.
const CLASS_NAME: PCWSTR = w!("StasisHarnessWindow");
const TITLE: PCWSTR = w!("Stasis harness (pretend this is a game)");

extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wp, lp) },
    }
}

fn create_window() -> Result<HWND> {
    unsafe {
        let instance: HINSTANCE = GetModuleHandleW(None)?.into();

        let class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            hInstance: instance,
            lpszClassName: CLASS_NAME,
            ..Default::default()
        };
        RegisterClassExW(&class);

        let hwnd = CreateWindowExW(
            Default::default(),
            CLASS_NAME,
            TITLE,
            WS_OVERLAPPEDWINDOW,
            100,
            100,
            WIDTH as i32,
            HEIGHT as i32,
            None,
            None,
            instance,
            None,
        )?;

        let _ = ShowWindow(hwnd, SW_SHOW);
        Ok(hwnd)
    }
}

struct Renderer {
    swapchain: IDXGISwapChain,
    context: ID3D11DeviceContext,
    target: ID3D11RenderTargetView,
}

fn create_renderer(hwnd: HWND) -> Result<Renderer> {
    // DXGI_SWAP_EFFECT_DISCARD is the old blit model rather than the flip model
    // most current games use. It is chosen on purpose: it is the harder case for
    // a Present hook to sit in front of, so a hook proven here is not relying on
    // flip-model behaviour it will not always get.
    let desc = DXGI_SWAP_CHAIN_DESC {
        BufferDesc: DXGI_MODE_DESC {
            Width: WIDTH,
            Height: HEIGHT,
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            ..Default::default()
        },
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: 2,
        OutputWindow: hwnd,
        Windowed: true.into(),
        SwapEffect: DXGI_SWAP_EFFECT_DISCARD,
        ..Default::default()
    };

    let mut swapchain: Option<IDXGISwapChain> = None;
    let mut device: Option<ID3D11Device> = None;
    let mut context: Option<ID3D11DeviceContext> = None;

    unsafe {
        D3D11CreateDeviceAndSwapChain(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            None,
            Default::default(),
            Some(&[D3D_FEATURE_LEVEL_11_0]),
            D3D11_SDK_VERSION,
            Some(&desc),
            Some(&mut swapchain),
            Some(&mut device),
            None,
            Some(&mut context),
        )?;
    }

    let swapchain = swapchain.expect("swapchain when creation succeeded");
    let device = device.expect("device when creation succeeded");
    let context = context.expect("context when creation succeeded");

    let back_buffer: ID3D11Texture2D = unsafe { swapchain.GetBuffer(0)? };
    let mut target: Option<ID3D11RenderTargetView> = None;
    unsafe { device.CreateRenderTargetView(&back_buffer, None, Some(&mut target))? };
    let target = target.expect("render target view when creation succeeded");

    Ok(Renderer {
        swapchain,
        context,
        target,
    })
}

fn main() -> Result<()> {
    let hwnd = create_window()?;
    let renderer = create_renderer(hwnd)?;

    // The injector and the hook both need these. Printing them means a human
    // running the harness by hand has everything the tooling asks for.
    let swapchain_ptr = renderer.swapchain.as_raw() as *const c_void;
    println!("harness pid    : {}", std::process::id());
    println!("harness hwnd   : {:?}", hwnd.0);
    println!("swapchain      : {swapchain_ptr:p}");
    println!("presenting. close the window to stop.");

    let mut frame: u32 = 0;
    let mut message = MSG::default();

    loop {
        unsafe {
            while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                if message.message == windows::Win32::UI::WindowsAndMessaging::WM_QUIT {
                    return Ok(());
                }
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }

            // A slowly cycling colour, so a present that has stopped is visible
            // without reading a log.
            let t = frame as f32 * 0.01;
            let clear = [0.05 + 0.05 * t.sin(), 0.05, 0.08 + 0.05 * t.cos(), 1.0f32];
            renderer
                .context
                .ClearRenderTargetView(&renderer.target, &clear);

            // Vsync on, so the loop runs at display rate rather than spinning a
            // core. The hook's per-frame cost is measured against this.
            renderer.swapchain.Present(1, DXGI_PRESENT(0)).ok()?;
        }

        frame = frame.wrapping_add(1);
    }
}
