use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};

/// Never activate on show or click, and stay out of Alt+Tab.
pub fn make_overlay_unobtrusive(window: &tauri::WebviewWindow) {
    let Ok(handle) = window.hwnd() else { return };
    let hwnd = HWND(handle.0 as _);
    // SAFETY: the handle belongs to a live window owned by this process.
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | (WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0) as isize);
    }
}
