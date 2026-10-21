pub mod icon;
pub mod resize;
#[cfg(target_os = "linux")]
pub mod shape_x11;

use crate::config::Settings;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use tiny_skia::Pixmap;
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
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGColorSpaceCreateDeviceRGB() -> *mut std::ffi::c_void;
    fn CGColorSpaceRelease(cs: *mut std::ffi::c_void);
    fn CGBitmapContextCreate(
        data: *mut std::ffi::c_void,
        width: usize,
        height: usize,
        bits_per_component: usize,
        bytes_per_row: usize,
        space: *mut std::ffi::c_void,
        bitmap_info: u32,
    ) -> *mut std::ffi::c_void;
    fn CGBitmapContextCreateImage(context: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    fn CGContextRelease(c: *mut std::ffi::c_void);
    fn CGImageRelease(image: *mut std::ffi::c_void);
}

#[cfg(target_os = "macos")]
fn apply_macos_window_tweaks(window: &Window, settings: &Settings) {
    use objc2::msg_send;
    use objc2::runtime::{AnyObject, Bool};
    use objc2_app_kit::{NSColor, NSView};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use std::ffi::c_void;

    let Ok(handle) = window.window_handle() else {
        return;
    };
    if let RawWindowHandle::AppKit(appkit) = handle.as_raw() {
        unsafe {
            let ns_view_ptr = appkit.ns_view.as_ptr() as *const NSView;
            let ns_view: &NSView = &*ns_view_ptr;
            ns_view.setWantsLayer(true);

            let layer: *mut AnyObject = msg_send![ns_view, layer];
            if !layer.is_null() {
                let _: () = msg_send![layer, setOpaque: Bool::NO];
                let _: () = msg_send![layer, setBackgroundColor: std::ptr::null::<c_void>()];
            }

            if let Some(ns_window) = ns_view.window() {
                ns_window.setOpaque(false);
                let clear = NSColor::clearColor();
                ns_window.setBackgroundColor(Some(&clear));
                ns_window.setHasShadow(!settings.appearance.shaped);
                ns_window.invalidateShadow();
                ns_window.setIgnoresMouseEvents(settings.window.click_through);
            }
        }
    }
}

/// Presents a premultiplied RGBA8 `Pixmap` directly to the macOS `NSView`'s backing `CALayer`
/// using a `CGImage` with `kCGImageAlphaPremultipliedLast`, preserving full per-pixel alpha
/// transparency (unlike `softbuffer`, which forces `kCGImageAlphaNoneSkipFirst` and opaque black).
#[cfg(target_os = "macos")]
pub fn present_macos_pixmap(window: &Window, pixmap: &Pixmap, settings: &Settings) {
    use objc2::msg_send;
    use objc2::runtime::{AnyClass, AnyObject, Bool};
    use objc2_app_kit::{NSColor, NSView};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use std::ffi::c_void;

    let width = pixmap.width() as usize;
    let height = pixmap.height() as usize;
    if width == 0 || height == 0 {
        return;
    }

    let Ok(handle) = window.window_handle() else {
        return;
    };
    let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
        return;
    };

    // kCGImageAlphaPremultipliedLast (1) | kCGBitmapByteOrder32Big (4 << 12)
    const BITMAP_INFO_RGBA_PREMUL: u32 = 1 | (4 << 12);

    unsafe {
        let color_space = CGColorSpaceCreateDeviceRGB();
        if color_space.is_null() {
            return;
        }

        let ctx = CGBitmapContextCreate(
            pixmap.data().as_ptr() as *mut c_void,
            width,
            height,
            8,
            width * 4,
            color_space,
            BITMAP_INFO_RGBA_PREMUL,
        );
        if ctx.is_null() {
            CGColorSpaceRelease(color_space);
            return;
        }

        let cg_image = CGBitmapContextCreateImage(ctx);
        CGContextRelease(ctx);
        CGColorSpaceRelease(color_space);

        if cg_image.is_null() {
            return;
        }

        let ns_view_ptr = appkit.ns_view.as_ptr() as *const NSView;
        let ns_view: &NSView = &*ns_view_ptr;
        ns_view.setWantsLayer(true);

        if let Some(ns_window) = ns_view.window() {
            ns_window.setOpaque(false);
            let clear = NSColor::clearColor();
            ns_window.setBackgroundColor(Some(&clear));
            ns_window.setHasShadow(!settings.appearance.shaped);
            ns_window.setIgnoresMouseEvents(settings.window.click_through);
        }

        let ca_transaction = AnyClass::get("CATransaction");
        if let Some(cls) = ca_transaction {
            let _: () = msg_send![cls, begin];
            let _: () = msg_send![cls, setDisableActions: Bool::YES];
        }

        let layer: *mut AnyObject = msg_send![ns_view, layer];
        if !layer.is_null() {
            let _: () = msg_send![layer, setOpaque: Bool::NO];
            let _: () = msg_send![layer, setBackgroundColor: std::ptr::null::<c_void>()];
            let scale: f64 = window.scale_factor().max(1.0);
            let _: () = msg_send![layer, setContentsScale: scale];
            let _: () = msg_send![layer, setContents: cg_image as *mut AnyObject];
        }

        if let Some(cls) = ca_transaction {
            let _: () = msg_send![cls, commit];
        }

        CGImageRelease(cg_image);
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
            ex_style |= WS_EX_LAYERED as isize;
            if settings.window.click_through {
                ex_style |= WS_EX_TRANSPARENT as isize;
            } else {
                ex_style &= !(WS_EX_TRANSPARENT as isize);
            }
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style);
        }
    }
}

/// Presents a premultiplied RGBA8 `Pixmap` on Windows using `UpdateLayeredWindow` (`ULW_ALPHA` +
/// `AC_SRC_ALPHA`), providing true 8-bit per-pixel alpha transparency without any black background
/// or rectangular window border/shadow (unlike `softbuffer`'s `StretchDIBits`).
#[cfg(target_os = "windows")]
pub fn present_windows_pixmap(window: &Window, pixmap: &Pixmap, settings: &Settings) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use std::ffi::c_void;
    use windows_sys::Win32::Foundation::{HWND, POINT, SIZE};
    use windows_sys::Win32::Graphics::Gdi::{
        CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC,
        SelectObject, AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
        BLENDFUNCTION, DIB_RGB_COLORS,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, UpdateLayeredWindow, GWL_EXSTYLE, ULW_ALPHA,
        WS_EX_LAYERED, WS_EX_TRANSPARENT,
    };

    let w = pixmap.width();
    let h = pixmap.height();
    if w == 0 || h == 0 {
        return;
    }

    let Ok(handle) = window.window_handle() else {
        return;
    };
    let RawWindowHandle::Win32(win32) = handle.as_raw() else {
        return;
    };
    let hwnd = win32.hwnd.get() as HWND;

    unsafe {
        let mut ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        ex_style |= WS_EX_LAYERED as isize;
        if settings.window.click_through {
            ex_style |= WS_EX_TRANSPARENT as isize;
        } else {
            ex_style &= !(WS_EX_TRANSPARENT as isize);
        }
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style);

        let screen_dc = GetDC(std::mem::zeroed());
        let mem_dc = CreateCompatibleDC(screen_dc);
        if mem_dc == std::mem::zeroed() {
            ReleaseDC(std::mem::zeroed(), screen_dc);
            return;
        }

        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = w as i32;
        bmi.bmiHeader.biHeight = -(h as i32); // top-down DIB
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB;

        let mut bits_ptr: *mut c_void = std::ptr::null_mut();
        let hbmp = CreateDIBSection(
            mem_dc,
            &bmi,
            DIB_RGB_COLORS,
            &mut bits_ptr,
            std::mem::zeroed(),
            0,
        );

        if hbmp != std::mem::zeroed() && !bits_ptr.is_null() {
            let old_obj = SelectObject(mem_dc, hbmp);
            let dst = std::slice::from_raw_parts_mut(bits_ptr as *mut u32, (w * h) as usize);
            for (out, px) in dst.iter_mut().zip(pixmap.pixels()) {
                let r = px.red() as u32;
                let g = px.green() as u32;
                let b = px.blue() as u32;
                let a = px.alpha() as u32;
                // Keep alpha=1 on transparent interior/grip pixels when not click-through
                // so Win32 delivers mouse events across the window bounds.
                let effective_a = if a == 0 && !settings.window.click_through {
                    1
                } else {
                    a
                };
                *out = (effective_a << 24) | (r << 16) | (g << 8) | b;
            }

            let size = SIZE {
                cx: w as i32,
                cy: h as i32,
            };
            let pt_src = POINT { x: 0, y: 0 };
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };

            let _ = UpdateLayeredWindow(
                hwnd,
                screen_dc,
                std::ptr::null(),
                &size,
                mem_dc,
                &pt_src,
                0,
                &blend,
                ULW_ALPHA,
            );

            SelectObject(mem_dc, old_obj);
            DeleteObject(hbmp);
        }

        DeleteDC(mem_dc);
        ReleaseDC(std::mem::zeroed(), screen_dc);
    }
}
