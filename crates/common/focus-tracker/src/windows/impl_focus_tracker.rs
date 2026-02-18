use crate::{FocusTrackerConfig, FocusTrackerError, FocusTrackerResult, FocusedWindow};
use focus_tracker_core::IconConfig;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::debug;

#[cfg(feature = "async")]
use std::future::Future;

use super::utils;

use windows_sys::Win32::{
    Foundation::{HWND, WPARAM},
    Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC,
        DeleteObject, GetDIBits, SelectObject,
    },
    UI::WindowsAndMessaging::{
        DestroyIcon, GCLP_HICON, GCLP_HICONSM, GetClassLongPtrW, GetIconInfo, ICON_BIG, ICON_SMALL,
        ICONINFO, SendMessageW, WM_GETICON,
    },
};

#[derive(Debug, Clone)]
pub(crate) struct ImplFocusTracker {}

impl ImplFocusTracker {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

#[derive(Default)]
struct FocusState {
    hwnd: isize,
    process_id: u32,
    process_name: String,
    window_title: Option<String>,
}

impl FocusState {
    fn has_changed(&self, hwnd: HWND, process_id: u32, title: &Option<String>) -> bool {
        let hwnd_value = hwnd as isize;
        hwnd_value != self.hwnd
            || process_id != self.process_id
            || self.window_title.as_deref() != title.as_deref()
    }

    fn focus_changed(&self, hwnd: HWND) -> bool {
        hwnd as isize != self.hwnd
    }

    fn update(&mut self, hwnd: HWND, process_id: u32, process_name: String, title: Option<String>) {
        self.hwnd = hwnd as isize;
        self.process_id = process_id;
        self.process_name = process_name;
        self.window_title = title;
    }

    fn clear(&mut self) {
        self.hwnd = 0;
        self.process_id = 0;
        self.process_name.clear();
        self.window_title = None;
    }
}

#[inline]
fn should_stop(stop_signal: Option<&AtomicBool>) -> bool {
    stop_signal.is_some_and(|stop| stop.load(Ordering::Relaxed))
}

fn poll_focus_change(
    prev_state: &mut FocusState,
    icon_cache: &mut HashMap<String, Arc<image::RgbaImage>>,
    icon_config: &IconConfig,
) -> Option<FocusedWindow> {
    let Some(hwnd) = utils::get_foreground_window() else {
        if prev_state.hwnd != 0 {
            prev_state.clear();
            icon_cache.clear();
        }
        return None;
    };

    let (title, process_name) = match utils::get_window_info(hwnd) {
        Ok(info) => info,
        Err(e) => {
            debug!("Failed to get window info: {e}");
            return None;
        }
    };

    let process_id = utils::get_window_process_id(hwnd).unwrap_or_default();

    if !prev_state.has_changed(hwnd, process_id, &title) {
        return None;
    }

    let icon = if prev_state.focus_changed(hwnd) {
        resolve_icon(icon_cache, hwnd, process_id, &process_name, icon_config)
    } else {
        icon_cache.get(&process_name).map(Arc::clone)
    };

    let focused = FocusedWindow {
        process_id,
        process_name: process_name.clone(),
        window_title: title.clone(),
        icon,
    };

    prev_state.update(hwnd, process_id, process_name, title);

    Some(focused)
}

impl ImplFocusTracker {
    pub fn track_focus<F>(&self, on_focus: F, config: &FocusTrackerConfig) -> FocusTrackerResult<()>
    where
        F: FnMut(FocusedWindow) -> FocusTrackerResult<()>,
    {
        self.run(on_focus, None, config)
    }

    pub fn track_focus_with_stop<F>(
        &self,
        on_focus: F,
        stop_signal: &AtomicBool,
        config: &FocusTrackerConfig,
    ) -> FocusTrackerResult<()>
    where
        F: FnMut(FocusedWindow) -> FocusTrackerResult<()>,
    {
        self.run(on_focus, Some(stop_signal), config)
    }

    #[cfg(feature = "async")]
    pub async fn track_focus_async<F, Fut>(
        &self,
        on_focus: F,
        config: &FocusTrackerConfig,
    ) -> FocusTrackerResult<()>
    where
        F: FnMut(FocusedWindow) -> Fut,
        Fut: Future<Output = FocusTrackerResult<()>>,
    {
        self.run_async(on_focus, None, config).await
    }

    #[cfg(feature = "async")]
    pub async fn track_focus_async_with_stop<F, Fut>(
        &self,
        on_focus: F,
        stop_signal: &AtomicBool,
        config: &FocusTrackerConfig,
    ) -> FocusTrackerResult<()>
    where
        F: FnMut(FocusedWindow) -> Fut,
        Fut: Future<Output = FocusTrackerResult<()>>,
    {
        self.run_async(on_focus, Some(stop_signal), config).await
    }

    #[cfg(feature = "async")]
    async fn run_async<F, Fut>(
        &self,
        mut on_focus: F,
        stop_signal: Option<&AtomicBool>,
        config: &FocusTrackerConfig,
    ) -> FocusTrackerResult<()>
    where
        F: FnMut(FocusedWindow) -> Fut,
        Fut: Future<Output = FocusTrackerResult<()>>,
    {
        if !utils::is_interactive_session()? {
            return Err(FocusTrackerError::NotInteractiveSession);
        }

        let mut prev_state = FocusState::default();
        let mut icon_cache: HashMap<String, Arc<image::RgbaImage>> = HashMap::new();

        loop {
            if should_stop(stop_signal) {
                debug!("Stop signal received, exiting focus tracking loop");
                break;
            }

            let pending = poll_focus_change(&mut prev_state, &mut icon_cache, &config.icon);

            if let Some(focused) = pending {
                on_focus(focused).await?;
            }

            tokio::time::sleep(config.poll_interval).await;
        }

        Ok(())
    }

    #[allow(clippy::unused_self)] // &self required for cross-platform API consistency
    fn run<F>(
        &self,
        mut on_focus: F,
        stop_signal: Option<&AtomicBool>,
        config: &FocusTrackerConfig,
    ) -> FocusTrackerResult<()>
    where
        F: FnMut(FocusedWindow) -> FocusTrackerResult<()>,
    {
        if !utils::is_interactive_session()? {
            return Err(FocusTrackerError::NotInteractiveSession);
        }

        let mut prev_state = FocusState::default();
        let mut icon_cache: HashMap<String, Arc<image::RgbaImage>> = HashMap::new();

        loop {
            if should_stop(stop_signal) {
                debug!("Stop signal received, exiting focus tracking loop");
                break;
            }

            if let Some(focused) = poll_focus_change(&mut prev_state, &mut icon_cache, &config.icon)
            {
                on_focus(focused)?;
            }

            std::thread::sleep(config.poll_interval);
        }

        Ok(())
    }
}

fn resolve_icon(
    cache: &mut HashMap<String, Arc<image::RgbaImage>>,
    hwnd: HWND,
    process_id: u32,
    process_name: &str,
    icon_config: &IconConfig,
) -> Option<Arc<image::RgbaImage>> {
    if let Some(cached) = cache.get(process_name) {
        return Some(Arc::clone(cached));
    }

    let image = match extract_window_icon(hwnd, icon_config) {
        Ok(img) => img,
        Err(e) => {
            debug!("Failed to extract window icon: {e}, trying exe fallback");
            match extract_exe_icon(process_id, icon_config) {
                Ok(img) => img,
                Err(e2) => {
                    debug!("Failed to extract exe icon: {e2}");
                    return None;
                }
            }
        }
    };

    let icon = Arc::new(image);
    cache.insert(process_name.to_owned(), Arc::clone(&icon));
    Some(icon)
}

struct DcGuard(windows_sys::Win32::Graphics::Gdi::HDC);

impl Drop for DcGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { DeleteDC(self.0) };
        }
    }
}

struct IconBitmapGuard {
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    old_bitmap: windows_sys::Win32::Graphics::Gdi::HGDIOBJ,
    hbm_color: windows_sys::Win32::Graphics::Gdi::HBITMAP,
    hbm_mask: windows_sys::Win32::Graphics::Gdi::HBITMAP,
}

impl Drop for IconBitmapGuard {
    fn drop(&mut self) {
        unsafe {
            if !self.hdc.is_null() {
                SelectObject(self.hdc, self.old_bitmap);
            }
            if !self.hbm_color.is_null() {
                DeleteObject(self.hbm_color);
            }
            if !self.hbm_mask.is_null() {
                DeleteObject(self.hbm_mask);
            }
        }
    }
}

fn acquire_icon_handle(hwnd: HWND) -> Result<(isize, bool), FocusTrackerError> {
    let hicon = unsafe { SendMessageW(hwnd, WM_GETICON, ICON_BIG as WPARAM, 0) } as isize;
    if hicon != 0 {
        return Ok((hicon, false));
    }

    let hicon = unsafe { SendMessageW(hwnd, WM_GETICON, ICON_SMALL as WPARAM, 0) } as isize;
    if hicon != 0 {
        return Ok((hicon, false));
    }

    let hicon = unsafe { GetClassLongPtrW(hwnd, GCLP_HICON) } as isize;
    if hicon != 0 {
        return Ok((hicon, false));
    }

    let hicon = unsafe { GetClassLongPtrW(hwnd, GCLP_HICONSM) } as isize;
    if hicon != 0 {
        return Ok((hicon, false));
    }

    Err(FocusTrackerError::platform("no icon found for window"))
}

fn extract_window_icon(
    hwnd: HWND,
    icon_config: &IconConfig,
) -> FocusTrackerResult<image::RgbaImage> {
    let (hicon, owned) = acquire_icon_handle(hwnd)?;
    let result = icon_handle_to_image(hicon, icon_config);

    if owned {
        unsafe { DestroyIcon(hicon as _) };
    }

    result
}

fn extract_exe_icon(
    process_id: u32,
    icon_config: &IconConfig,
) -> FocusTrackerResult<image::RgbaImage> {
    use windows_sys::Win32::UI::Shell::ExtractIconExW;

    let exe_path = utils::get_process_exe_path(process_id)?;

    let mut path_z = exe_path;
    path_z.push(0);

    let mut hicon_large = std::ptr::null_mut();
    let count = unsafe {
        ExtractIconExW(
            path_z.as_ptr(),
            0,
            &mut hicon_large,
            std::ptr::null_mut(),
            1,
        )
    };

    if count == 0 || hicon_large.is_null() {
        return Err(FocusTrackerError::platform(
            "no icon found in process executable",
        ));
    }

    let result = icon_handle_to_image(hicon_large as isize, icon_config);

    unsafe { DestroyIcon(hicon_large as _) };

    result
}

fn icon_handle_to_image(
    hicon: isize,
    icon_config: &IconConfig,
) -> FocusTrackerResult<image::RgbaImage> {
    let mut icon_info: ICONINFO = unsafe { std::mem::zeroed() };
    if unsafe { GetIconInfo(hicon as _, &mut icon_info) } == 0 {
        return Err(FocusTrackerError::platform("failed to get icon info"));
    }

    let bitmap = if !icon_info.hbmColor.is_null() {
        icon_info.hbmColor
    } else {
        icon_info.hbmMask
    };

    let hdc = unsafe { CreateCompatibleDC(std::ptr::null_mut()) };
    if hdc.is_null() {
        unsafe {
            if !icon_info.hbmColor.is_null() {
                DeleteObject(icon_info.hbmColor);
            }
            if !icon_info.hbmMask.is_null() {
                DeleteObject(icon_info.hbmMask);
            }
        }
        return Err(FocusTrackerError::platform("failed to create DC"));
    }

    let _dc_guard = DcGuard(hdc);
    let old_bitmap = unsafe { SelectObject(hdc, bitmap) };
    let _bmp_guard = IconBitmapGuard {
        hdc,
        old_bitmap,
        hbm_color: icon_info.hbmColor,
        hbm_mask: icon_info.hbmMask,
    };

    let mut bmi: BITMAPINFO = unsafe { std::mem::zeroed() };
    bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;

    if unsafe {
        GetDIBits(
            hdc,
            bitmap,
            0,
            0,
            std::ptr::null_mut(),
            &mut bmi,
            DIB_RGB_COLORS,
        )
    } == 0
    {
        return Err(FocusTrackerError::platform("failed to get bitmap info"));
    }

    let width = bmi.bmiHeader.biWidth as u32;
    let height = bmi.bmiHeader.biHeight.unsigned_abs();

    if width == 0 || height == 0 {
        return Err(FocusTrackerError::platform("invalid icon dimensions"));
    }

    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB;
    bmi.bmiHeader.biHeight = -(height as i32);

    let pixel_count = (width * height) as usize;
    let mut pixels: Vec<u8> = vec![0; pixel_count * 4];

    if unsafe {
        GetDIBits(
            hdc,
            bitmap,
            0,
            height,
            pixels.as_mut_ptr() as *mut _,
            &mut bmi,
            DIB_RGB_COLORS,
        )
    } == 0
    {
        return Err(FocusTrackerError::platform("failed to get bitmap bits"));
    }

    for i in (0..pixels.len()).step_by(4) {
        pixels.swap(i, i + 2);
    }

    let mut image = image::RgbaImage::from_raw(width, height, pixels)
        .ok_or_else(|| FocusTrackerError::platform("failed to create RgbaImage from pixel data"))?;

    if let Some(target_size) = icon_config.size
        && (image.width() != target_size || image.height() != target_size)
    {
        image = image::imageops::resize(&image, target_size, target_size, icon_config.filter_type);
    }

    Ok(image)
}
