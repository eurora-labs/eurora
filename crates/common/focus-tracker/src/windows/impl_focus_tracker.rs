use crate::{FocusTrackerConfig, FocusTrackerError, FocusTrackerResult, FocusedWindow};
use focus_tracker_core::IconConfig;
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(feature = "async")]
use std::future::Future;
use windows_sys::Win32::{
    Foundation::{HWND, WPARAM},
    Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC,
        DeleteObject, GetDIBits, SelectObject,
    },
    UI::WindowsAndMessaging::{
        GCLP_HICON, GCLP_HICONSM, GetClassLongPtrW, ICON_BIG, ICON_SMALL, SendMessageW, WM_GETICON,
    },
};

use super::utils;
use tracing::info;

#[derive(Debug, Clone)]
pub(crate) struct ImplFocusTracker {}

impl ImplFocusTracker {
    pub(crate) fn new() -> Self {
        Self {}
    }
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

        // Store HWND as isize to satisfy Send for async contexts
        let mut prev_hwnd: Option<isize> = None;
        let mut prev_title: Option<String> = None;
        let mut cached_icon: Option<image::RgbaImage> = None;

        if let Some(window_info) = get_window_info_without_icon() {
            let icon = get_window_icon_by_hwnd(window_info.hwnd_value, &config.icon);
            cached_icon = icon.clone();

            let focused_window = FocusedWindow {
                process_id: window_info.process_id,
                process_name: window_info.process_name,
                window_title: window_info.window_title.clone(),
                icon,
            };

            if let Err(e) = on_focus(focused_window).await {
                info!("Focus event handler failed: {}", e);
            }

            prev_hwnd = Some(window_info.hwnd_value);
            prev_title = window_info.window_title;
        }

        loop {
            if let Some(stop) = stop_signal
                && stop.load(Ordering::Relaxed)
            {
                break;
            }

            if let Some(window_info) = get_window_info_without_icon() {
                let current_hwnd_value = window_info.hwnd_value;
                let focus_changed = match prev_hwnd {
                    Some(prev) => prev != current_hwnd_value,
                    None => true,
                };

                let title_changed = match &prev_title {
                    Some(prev_t) => Some(prev_t) != window_info.window_title.as_ref(),
                    None => true,
                };

                if focus_changed || title_changed {
                    let icon = if focus_changed {
                        let new_icon = get_window_icon_by_hwnd(current_hwnd_value, &config.icon);
                        cached_icon = new_icon.clone();
                        new_icon
                    } else {
                        cached_icon.clone()
                    };

                    let focused_window = FocusedWindow {
                        process_id: window_info.process_id,
                        process_name: window_info.process_name,
                        window_title: window_info.window_title.clone(),
                        icon,
                    };

                    if let Err(e) = on_focus(focused_window).await {
                        info!("Focus event handler failed: {}", e);
                    }

                    prev_hwnd = Some(current_hwnd_value);
                    prev_title = window_info.window_title;
                }
            } else {
                if prev_hwnd.is_some() {
                    prev_hwnd = None;
                    prev_title = None;
                    cached_icon = None;
                }
            }

            tokio::time::sleep(config.poll_interval).await;
        }

        Ok(())
    }

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

        let mut prev_hwnd: Option<isize> = None;
        let mut prev_title: Option<String> = None;
        let mut cached_icon: Option<image::RgbaImage> = None;

        if let Some(hwnd) = utils::get_foreground_window()
            && let Ok((title, process)) = unsafe { utils::get_window_info(hwnd) }
        {
            let icon = get_window_icon(hwnd, &config.icon);
            cached_icon = icon.clone();

            let process_id = unsafe { utils::get_window_process_id(hwnd) }.unwrap_or_default();
            if let Err(e) = on_focus(FocusedWindow {
                process_id,
                process_name: process.clone(),
                window_title: Some(title.clone()),
                icon,
            }) {
                info!("Focus event handler failed: {}", e);
            }

            prev_hwnd = Some(hwnd as isize);
            prev_title = Some(title);
        }

        loop {
            if let Some(stop) = stop_signal
                && stop.load(Ordering::Relaxed)
            {
                break;
            }

            if let Some(current_hwnd) = utils::get_foreground_window() {
                let current_hwnd_value = current_hwnd as isize;
                let focus_changed = match prev_hwnd {
                    Some(prev) => prev != current_hwnd_value,
                    None => true,
                };

                match unsafe { utils::get_window_info(current_hwnd) } {
                    Ok((title, process)) => {
                        // Also check if title changed for the same window
                        let title_changed = match &prev_title {
                            Some(prev_t) => prev_t != &title,
                            None => true,
                        };

                        // Trigger handler if either window focus or title has changed
                        if focus_changed || title_changed {
                            // Only fetch icon when the focused app changes, not on title changes
                            let icon = if focus_changed {
                                let new_icon = get_window_icon(current_hwnd, &config.icon);
                                cached_icon = new_icon.clone();
                                new_icon
                            } else {
                                cached_icon.clone()
                            };

                            let process_id = unsafe { utils::get_window_process_id(current_hwnd) }
                                .unwrap_or_default();
                            if let Err(e) = on_focus(FocusedWindow {
                                process_id,
                                process_name: process.clone(),
                                window_title: Some(title.clone()),
                                icon,
                            }) {
                                info!("Focus event handler failed: {}", e);
                            }

                            prev_hwnd = Some(current_hwnd_value);
                            prev_title = Some(title);
                        }
                    }
                    Err(e) => {
                        info!("Failed to get window info: {}", e);
                    }
                }
            } else {
                if prev_hwnd.is_some() {
                    prev_hwnd = None;
                    prev_title = None;
                    cached_icon = None;
                }
            }

            std::thread::sleep(config.poll_interval);
        }

        Ok(())
    }
}

struct WindowInfo {
    hwnd_value: isize,
    process_id: u32,
    process_name: String,
    window_title: Option<String>,
}

fn get_window_info_without_icon() -> Option<WindowInfo> {
    let hwnd = utils::get_foreground_window()?;
    let hwnd_value = hwnd as isize;

    let (title, process) = unsafe { utils::get_window_info(hwnd) }.ok()?;
    let process_id = unsafe { utils::get_window_process_id(hwnd) }.unwrap_or_default();

    Some(WindowInfo {
        hwnd_value,
        process_id,
        process_name: process,
        window_title: Some(title),
    })
}

fn get_window_icon_by_hwnd(
    hwnd_value: isize,
    icon_config: &IconConfig,
) -> Option<image::RgbaImage> {
    get_window_icon(hwnd_value as HWND, icon_config)
}

fn resize_icon(
    image: image::RgbaImage,
    target_size: u32,
    filter_type: image::imageops::FilterType,
) -> image::RgbaImage {
    if image.width() == target_size && image.height() == target_size {
        return image;
    }

    image::imageops::resize(&image, target_size, target_size, filter_type)
}

fn get_window_icon(hwnd: HWND, icon_config: &IconConfig) -> Option<image::RgbaImage> {
    unsafe { extract_window_icon(hwnd, icon_config).ok() }
}

unsafe fn extract_window_icon(
    hwnd: HWND,
    icon_config: &IconConfig,
) -> FocusTrackerResult<image::RgbaImage> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, ICONINFO};

    let hicon = unsafe { SendMessageW(hwnd, WM_GETICON, ICON_BIG as WPARAM, 0) };
    let hicon = if hicon != 0 {
        hicon as isize
    } else {
        let hicon = unsafe { SendMessageW(hwnd, WM_GETICON, ICON_SMALL as WPARAM, 0) };
        if hicon != 0 {
            hicon as isize
        } else {
            let hicon = unsafe { GetClassLongPtrW(hwnd, GCLP_HICON) } as isize;
            if hicon != 0 {
                hicon
            } else {
                let hicon = unsafe { GetClassLongPtrW(hwnd, GCLP_HICONSM) } as isize;
                if hicon != 0 {
                    hicon
                } else {
                    return Err(FocusTrackerError::Platform(
                        "No icon found for window".to_string(),
                    ));
                }
            }
        }
    };

    let mut icon_info: ICONINFO = unsafe { std::mem::zeroed() };
    if unsafe { GetIconInfo(hicon as _, &mut icon_info) } == 0 {
        return Err(FocusTrackerError::Platform(
            "Failed to get icon info".to_string(),
        ));
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
        return Err(FocusTrackerError::Platform(
            "Failed to create DC".to_string(),
        ));
    }

    let old_bitmap = unsafe { SelectObject(hdc, bitmap) };

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
        unsafe {
            SelectObject(hdc, old_bitmap);
            DeleteDC(hdc);
            if !icon_info.hbmColor.is_null() {
                DeleteObject(icon_info.hbmColor);
            }
            if !icon_info.hbmMask.is_null() {
                DeleteObject(icon_info.hbmMask);
            }
        }
        return Err(FocusTrackerError::Platform(
            "Failed to get bitmap info".to_string(),
        ));
    }

    let width = bmi.bmiHeader.biWidth as u32;
    let height = bmi.bmiHeader.biHeight.unsigned_abs();

    if width == 0 || height == 0 {
        unsafe {
            SelectObject(hdc, old_bitmap);
            DeleteDC(hdc);
            if !icon_info.hbmColor.is_null() {
                DeleteObject(icon_info.hbmColor);
            }
            if !icon_info.hbmMask.is_null() {
                DeleteObject(icon_info.hbmMask);
            }
        }
        return Err(FocusTrackerError::Platform(
            "Invalid icon dimensions".to_string(),
        ));
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
        unsafe {
            SelectObject(hdc, old_bitmap);
            DeleteDC(hdc);
            if !icon_info.hbmColor.is_null() {
                DeleteObject(icon_info.hbmColor);
            }
            if !icon_info.hbmMask.is_null() {
                DeleteObject(icon_info.hbmMask);
            }
        }
        return Err(FocusTrackerError::Platform(
            "Failed to get bitmap bits".to_string(),
        ));
    }

    for i in (0..pixels.len()).step_by(4) {
        pixels.swap(i, i + 2);
    }

    unsafe {
        SelectObject(hdc, old_bitmap);
        DeleteDC(hdc);
        if !icon_info.hbmColor.is_null() {
            DeleteObject(icon_info.hbmColor);
        }
        if !icon_info.hbmMask.is_null() {
            DeleteObject(icon_info.hbmMask);
        }
        // Only destroy icons from WM_GETICON; GetClassLongPtrW icons are owned by the class
        let from_wm_geticon = SendMessageW(hwnd, WM_GETICON, ICON_BIG as WPARAM, 0) != 0
            || SendMessageW(hwnd, WM_GETICON, ICON_SMALL as WPARAM, 0) != 0;
        if from_wm_geticon {
            DestroyIcon(hicon as _);
        }
    }

    let mut image = image::RgbaImage::from_raw(width, height, pixels).ok_or_else(|| {
        FocusTrackerError::Platform("Failed to create RgbaImage from pixel data".to_string())
    })?;

    if let Some(target_size) = icon_config.size {
        image = resize_icon(image, target_size, icon_config.filter_type);
    }

    Ok(image)
}
