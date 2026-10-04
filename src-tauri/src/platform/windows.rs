use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOOWNERZORDER, SWP_NOSIZE, WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};

/// Never activate on show or click, and stay out of Alt+Tab and the taskbar.
/// Tao sets WS_EX_APPWINDOW, which would otherwise force a taskbar button.
pub fn make_overlay_unobtrusive(window: &tauri::WebviewWindow) {
    let Ok(handle) = window.hwnd() else { return };
    let hwnd = HWND(handle.0 as _);
    // SAFETY: the handle belongs to a live window owned by this process.
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let unobtrusive = (style | (WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0) as isize) & !(WS_EX_APPWINDOW.0 as isize);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, unobtrusive);
    }
}

/// A topmost window activated after ours (Task Manager on top, a pinned video)
/// covers the buddy. Re-raising to the top of the topmost band fixes that.
pub fn keep_overlay_on_top(window: &tauri::WebviewWindow) {
    let Ok(handle) = window.hwnd() else { return };
    // SAFETY: the handle belongs to a live window owned by this process.
    unsafe {
        let _ = SetWindowPos(
            HWND(handle.0 as _),
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
        );
    }
}
