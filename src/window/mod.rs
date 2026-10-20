pub mod resize;
#[cfg(target_os = "linux")]
pub mod shape_x11;

use crate::config::Settings;
use winit::window::{Window, WindowLevel};

/// Applies runtime window attributes (`always_on_top`, `decorations`, `click_through`, platform shadows).
pub fn apply_window_attributes(window: &Window, settings: &Settings) {
    window.set_decorations(settings.window.decorations);
    window.set_window_level(if settings.window.always_on_top {
        WindowLevel::AlwaysOnTop
    } else {
        WindowLevel::Normal
    });
    let _ = window.set_cursor_hittest(!settings.window.click_through);

    #[cfg(target_os = "macos")]
    apply_macos_window_tweaks(window, settings);

    #[cfg(target_os = "windows")]
    apply_windows_window_tweaks(window, settings);
}

#[cfg(target_os = "macos")]
fn apply_macos_window_tweaks(window: &Window, settings: &Settings) {
    use objc2_app_kit::{NSColor, NSView};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let Ok(handle) = window.window_handle() else {
        return;
    };
    if let RawWindowHandle::AppKit(appkit) = handle.as_raw() {
        unsafe {
            let ns_view_ptr = appkit.ns_view.as_ptr() as *const NSView;
            let ns_view: &NSView = &*ns_view_ptr;
            if let Some(ns_window) = ns_view.window() {
                ns_window.setOpaque(!settings.appearance.shaped);
                if settings.appearance.shaped {
                    let clear = NSColor::clearColor();
                    ns_window.setBackgroundColor(Some(&clear));
                    ns_window.setHasShadow(false);
                } else {
                    ns_window.setHasShadow(true);
                }
                ns_window.setIgnoresMouseEvents(settings.window.click_through);
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn apply_windows_window_tweaks(window: &Window, settings: &Settings) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_LAYERED, WS_EX_TRANSPARENT,
    };

    let Ok(handle) = window.window_handle() else {
        return;
    };
    if let RawWindowHandle::Win32(win32) = handle.as_raw() {
        let hwnd = win32.hwnd.get() as HWND;
        unsafe {
            let mut ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            if settings.window.click_through {
                ex_style |= (WS_EX_LAYERED | WS_EX_TRANSPARENT) as isize;
            } else {
                ex_style &= !(WS_EX_TRANSPARENT as isize);
            }
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style);
        }
    }
}
