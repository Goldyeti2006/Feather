use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GetClientRect,
    RegisterClassW, WNDCLASSW, WS_CHILD, WS_VISIBLE,
    CS_HREDRAW, CS_VREDRAW,
};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, EndPaint, FillRect, CreateSolidBrush,
    PAINTSTRUCT,
};
use windows::core::PCWSTR;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

pub const TOOLBAR_HEIGHT: i32 = 48;

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

pub unsafe fn create_toolbar(parent: HWND, width: i32) -> HWND {
    let class_name = to_wide("FeatherToolbar");

    let wc = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(toolbar_wnd_proc),
        hInstance: std::mem::transmute(
    windows::Win32::System::LibraryLoader::GetModuleHandleW(None).unwrap()
),
        lpszClassName: PCWSTR(class_name.as_ptr()),
        ..Default::default()
    };

    RegisterClassW(&wc);

    let class_wide = to_wide("FeatherToolbar");
    let title_wide = to_wide("");

    CreateWindowExW(
        Default::default(),
        PCWSTR(class_wide.as_ptr()),
        PCWSTR(title_wide.as_ptr()),
        WS_CHILD | WS_VISIBLE,
        0, 0,                    // x, y
        width, TOOLBAR_HEIGHT,   // width, height
        parent,
        None,
        windows::Win32::System::LibraryLoader::GetModuleHandleW(None)
            .unwrap(),
        None,
    ).unwrap()
}

unsafe extern "system" fn toolbar_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::WM_PAINT;

    match msg {
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            let mut rc = RECT::default();
            GetClientRect(hwnd, &mut rc).unwrap();
            // Draw dark toolbar background #1a1a1a
            let brush = CreateSolidBrush(windows::Win32::Foundation::COLORREF(0x001a1a1a));
            FillRect(hdc, &rc, brush);
            let _ = EndPaint(hwnd, &ps);
            windows::Win32::Foundation::LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}