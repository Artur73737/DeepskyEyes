//! Native Windows chrome helpers for the frameless app window.
//!
//! Only Win32 style bits are touched so the OS keeps resize borders, snap,
//! taskbar and DWM shadows. GPUI keeps owning its message loop: there is no
//! wndproc subclassing and no message interception here.
use windows::{
    core::HSTRING,
    Win32::{
        Foundation::{HWND, WPARAM},
        UI::{
            Input::KeyboardAndMouse::ReleaseCapture,
            WindowsAndMessaging::{
                FindWindowW, GetWindowLongW, GWL_STYLE, HTCAPTION, IsZoomed, SC_MOVE,
                SendMessageW, SetWindowLongW, SetWindowPos, ShowWindowAsync, SW_MAXIMIZE,
                SW_MINIMIZE, SW_RESTORE, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
                SWP_NOZORDER, WM_SYSCOMMAND, WS_CAPTION,
            },
        },
    },
};

/// Must match the native title set in `desktop::run`.
pub const WINDOW_TITLE: &str = "DeepskyEyes · Mission control";

fn find_own_window() -> Option<HWND> {
    unsafe {
        let name = HSTRING::from(WINDOW_TITLE);
        FindWindowW(None::<&windows::core::PCWSTR>, &name).ok()
    }
}

/// Remove the native caption, keeping the thick frame so OS resize, snap and
/// shadows keep working. Best effort: false when the window is not found.
pub fn strip_native_caption() -> bool {
    unsafe {
        let Some(hwnd) = find_own_window() else {
            return false;
        };
        let style = GetWindowLongW(hwnd, GWL_STYLE);
        let framed = style & !(WS_CAPTION.0 as i32);
        if framed == style {
            return true;
        }
        SetWindowLongW(hwnd, GWL_STYLE, framed);
        SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .is_ok()
    }
}

/// Poll for our window, then strip the caption once. Never panics.
pub fn strip_caption_when_ready() {
    std::thread::Builder::new()
        .name("deepsky-chrome".into())
        .spawn(|| {
            for _ in 0..100 {
                if strip_native_caption() {
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            eprintln!("deepsky-ui: native caption left in place (window not found)");
        })
        .ok();
}

/// Begin a title-bar drag (ReleaseCapture + SC_MOVE/HTCAPTION).
pub fn begin_drag() -> bool {
    unsafe {
        let Some(hwnd) = find_own_window() else {
            return false;
        };
        let _ = ReleaseCapture();
        SendMessageW(
            hwnd,
            WM_SYSCOMMAND,
            Some(WPARAM((SC_MOVE | HTCAPTION) as usize)),
            None,
        );
        true
    }
}

/// Minimize to the taskbar. (Kept beside toggle_maximize; the title bar
/// currently minimizes through GPUI's own window handle instead.)
#[allow(dead_code)]
pub fn minimize() -> bool {
    unsafe {
        find_own_window()
            .map(|hwnd| {
                let _ = ShowWindowAsync(hwnd, SW_MINIMIZE);
            })
            .is_some()
    }
}

/// Toggle maximized / restored. GPUI's own zoom only maximizes, hence raw Win32.
pub fn toggle_maximize() -> bool {
    unsafe {
        let Some(hwnd) = find_own_window() else {
            return false;
        };
        if IsZoomed(hwnd).as_bool() {
            let _ = ShowWindowAsync(hwnd, SW_RESTORE);
        } else {
            let _ = ShowWindowAsync(hwnd, SW_MAXIMIZE);
        }
        true
    }
}
