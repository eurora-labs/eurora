use focus_tracker_core::{FocusTrackerError, FocusTrackerResult, FocusedWindow, IconConfig};
use objc2::AnyThread;
use objc2::rc::autoreleasepool;
use objc2::runtime::AnyObject;
use objc2_app_kit::{
    NSBitmapImageFileType, NSBitmapImageRep, NSCalibratedRGBColorSpace, NSGraphicsContext, NSImage,
    NSRunningApplication, NSWorkspace,
};
use objc2_foundation::{NSDictionary, NSPoint, NSRect, NSSize, NSString, ns_string};
use std::ffi::c_void;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGWindowListCopyWindowInfo(option: u32, relative_to_window: u32) -> *const c_void;
}

const K_CG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY: u32 = 1;
const K_CG_WINDOW_LIST_EXCLUDE_DESKTOP_ELEMENTS: u32 = 1 << 4;
const K_CG_NULL_WINDOW_ID: u32 = 0;

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(cf: *const c_void);

    fn CFArrayGetCount(the_array: *const c_void) -> isize;
    fn CFArrayGetValueAtIndex(the_array: *const c_void, idx: isize) -> *const c_void;

    fn CFDictionaryGetValue(the_dict: *const c_void, key: *const c_void) -> *const c_void;

    fn CFNumberGetValue(number: *const c_void, the_type: i32, value_ptr: *mut c_void) -> bool;

    fn CFStringGetLength(the_string: *const c_void) -> isize;
    fn CFStringGetCString(
        the_string: *const c_void,
        buffer: *mut i8,
        buffer_size: isize,
        encoding: u32,
    ) -> bool;
}

const K_CF_NUMBER_SINT32_TYPE: i32 = 3;
const K_CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXUIElementCreateApplication(pid: i32) -> *mut c_void;
    fn AXUIElementCopyAttributeValue(
        element: *const c_void,
        attribute: *const c_void,
        value: *mut *mut c_void,
    ) -> i32;
}

const K_AX_ERROR_SUCCESS: i32 = 0;
const K_AX_ERROR_API_DISABLED: i32 = -25211;

/// Returns information about the currently focused (frontmost) window.
///
/// Uses [`CGWindowListCopyWindowInfo`] to query the window server directly
/// (reliable from any thread), then resolves the process name via
/// [`NSRunningApplication`] and the window title via the Accessibility API.
///
/// # Errors
///
/// Returns an error if no on-screen window is found, the process name cannot
/// be determined, or the Accessibility API denies permission.
pub fn get_frontmost_window_basic_info() -> FocusTrackerResult<FocusedWindow> {
    autoreleasepool(|_pool| {
        let pid = get_frontmost_window_pid()?;

        let app = NSRunningApplication::runningApplicationWithProcessIdentifier(pid);

        let process_name = app
            .and_then(|a| a.localizedName().map(|n| n.to_string()))
            .ok_or_else(|| {
                FocusTrackerError::platform(format!("failed to get process name for pid {pid}"))
            })?;

        let window_title = get_window_title_via_accessibility(pid)?;

        Ok(FocusedWindow {
            process_id: u32::try_from(pid).unwrap_or(0),
            window_title,
            process_name,
            icon: None,
        })
    })
}

/// Fetches the application icon for the given PID and returns it as an RGBA
/// image.
///
/// The icon is extracted via [`NSWorkspace::iconForFile`] using the app's
/// bundle path, then rendered at the target size through
/// [`NSGraphicsContext`] into an [`NSBitmapImageRep`] and encoded as PNG.
///
/// # Errors
///
/// Returns an error if the icon cannot be rendered or if the PNG data
/// cannot be decoded.
pub fn fetch_icon_for_pid(
    pid: i32,
    icon_config: &IconConfig,
) -> FocusTrackerResult<Option<image::RgbaImage>> {
    autoreleasepool(|_pool| {
        let app = NSRunningApplication::runningApplicationWithProcessIdentifier(pid);
        match app {
            Some(app) => get_app_icon(&app, icon_config),
            None => Ok(None),
        }
    })
}

/// Queries the window server for the PID of the frontmost normal application
/// window.
///
/// The window list returned by `CGWindowListCopyWindowInfo` is ordered
/// front-to-back.  We pick the first entry at layer 0 (normal windows),
/// which corresponds to the currently focused application.  Status-bar items,
/// menus, and other chrome live on higher layers and are skipped.
fn get_frontmost_window_pid() -> FocusTrackerResult<i32> {
    unsafe {
        let options =
            K_CG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY | K_CG_WINDOW_LIST_EXCLUDE_DESKTOP_ELEMENTS;
        let window_list = CGWindowListCopyWindowInfo(options, K_CG_NULL_WINDOW_ID);

        if window_list.is_null() {
            return Err(FocusTrackerError::platform("failed to get window list"));
        }

        let count = CFArrayGetCount(window_list);
        if count <= 0 {
            CFRelease(window_list);
            return Err(FocusTrackerError::platform("no windows found"));
        }

        let layer_key: *const c_void =
            std::ptr::from_ref::<NSString>(ns_string!("kCGWindowLayer")).cast();
        let pid_key: *const c_void =
            std::ptr::from_ref::<NSString>(ns_string!("kCGWindowOwnerPID")).cast();

        for i in 0..count {
            let dict = CFArrayGetValueAtIndex(window_list, i);

            let layer_val = CFDictionaryGetValue(dict, layer_key);
            if !layer_val.is_null() {
                let mut layer: i32 = 0;
                let ok = CFNumberGetValue(
                    layer_val,
                    K_CF_NUMBER_SINT32_TYPE,
                    std::ptr::from_mut(&mut layer).cast(),
                );
                if ok && layer != 0 {
                    continue;
                }
            }

            let pid_val = CFDictionaryGetValue(dict, pid_key);
            if pid_val.is_null() {
                continue;
            }
            let mut pid: i32 = 0;
            if !CFNumberGetValue(
                pid_val,
                K_CF_NUMBER_SINT32_TYPE,
                std::ptr::from_mut(&mut pid).cast(),
            ) {
                continue;
            }

            CFRelease(window_list);
            return Ok(pid);
        }

        CFRelease(window_list);
        Err(FocusTrackerError::platform(
            "no normal application window found",
        ))
    }
}

fn get_window_title_via_accessibility(pid: i32) -> FocusTrackerResult<Option<String>> {
    let app_element = unsafe { AXUIElementCreateApplication(pid) };
    if app_element.is_null() {
        return Ok(None);
    }

    let focused_window_attr = ns_string!("AXFocusedWindow");
    let mut focused_window: *mut c_void = std::ptr::null_mut();
    let result = unsafe {
        AXUIElementCopyAttributeValue(
            app_element,
            std::ptr::from_ref::<NSString>(focused_window_attr).cast::<c_void>(),
            &raw mut focused_window,
        )
    };

    unsafe { CFRelease(app_element) };

    if result == K_AX_ERROR_API_DISABLED {
        return Err(FocusTrackerError::PermissionDenied {
            context: "macOS accessibility API denied (AXUIElement)".into(),
        });
    }

    if result != K_AX_ERROR_SUCCESS || focused_window.is_null() {
        return Ok(None);
    }

    let title_attr = ns_string!("AXTitle");
    let mut title_value: *mut c_void = std::ptr::null_mut();
    let result = unsafe {
        AXUIElementCopyAttributeValue(
            focused_window,
            std::ptr::from_ref::<NSString>(title_attr).cast::<c_void>(),
            &raw mut title_value,
        )
    };

    unsafe { CFRelease(focused_window) };

    if result != K_AX_ERROR_SUCCESS || title_value.is_null() {
        return Ok(None);
    }

    let title_str = unsafe { cfstring_to_string(title_value) };
    unsafe { CFRelease(title_value) };

    Ok(title_str)
}

/// Converts a `CFStringRef` (passed as `*const c_void`) to a Rust [`String`].
///
/// # Safety
///
/// `cf_string` must point to a valid `CFString` instance, or be null.
unsafe fn cfstring_to_string(cf_string: *const c_void) -> Option<String> {
    if cf_string.is_null() {
        return None;
    }

    let length = unsafe { CFStringGetLength(cf_string) };
    if length <= 0 {
        return Some(String::new());
    }

    let buffer_size = (length * 4 + 1).cast_unsigned();
    let mut buffer: Vec<i8> = vec![0; buffer_size];

    let success = unsafe {
        CFStringGetCString(
            cf_string,
            buffer.as_mut_ptr(),
            buffer_size.cast_signed(),
            K_CF_STRING_ENCODING_UTF8,
        )
    };

    if success {
        let c_str = unsafe { std::ffi::CStr::from_ptr(buffer.as_ptr()) };
        c_str.to_str().ok().map(std::string::ToString::to_string)
    } else {
        None
    }
}

fn get_app_icon(
    app: &NSRunningApplication,
    icon_config: &IconConfig,
) -> FocusTrackerResult<Option<image::RgbaImage>> {
    let Some(bundle_url) = app.bundleURL() else {
        return Ok(None);
    };

    let Some(path) = bundle_url.path() else {
        return Ok(None);
    };

    let workspace = NSWorkspace::sharedWorkspace();
    let ns_image = workspace.iconForFile(&path);

    nsimage_to_rgba(&ns_image, icon_config)
}

/// Converts an [`NSImage`] to an [`image::RgbaImage`] at the configured icon
/// dimensions.
///
/// Instead of decoding the raw multi-resolution TIFF that `NSImage` produces,
/// we draw the image at the target size into a fresh [`NSBitmapImageRep`] via
/// [`NSGraphicsContext`].  This lets AppKit handle resolution selection and
/// colour-profile normalisation in one step and produces a small bitmap that
/// is fast to encode as PNG and decode with the [`image`] crate.
///
/// # Thread safety
///
/// `NSGraphicsContext::graphicsContextWithBitmapImageRep:` creates a purely
/// off-screen context that is safe to use from any thread (per Apple
/// documentation).
fn nsimage_to_rgba(
    ns_image: &NSImage,
    icon_config: &IconConfig,
) -> FocusTrackerResult<Option<image::RgbaImage>> {
    let icon_size = icon_config.get_size_or_default();

    let png_bytes = render_nsimage_to_png(ns_image, icon_size)?;

    let dynamic_image = image::load_from_memory(&png_bytes).map_err(|e| {
        FocusTrackerError::platform_with_source("failed to decode icon image data", e)
    })?;

    Ok(Some(dynamic_image.to_rgba8()))
}

/// Draws an [`NSImage`] at `size × size` pixels into a new RGBA
/// [`NSBitmapImageRep`] and returns the result encoded as PNG.
///
/// By rendering through [`NSGraphicsContext`] AppKit picks the best resolution
/// variant from the (potentially multi-resolution) source image and applies
/// any necessary colour-space conversions.  The output is a plain
/// `size × size` RGBA PNG that the [`image`] crate can decode without issues.
fn render_nsimage_to_png(ns_image: &NSImage, size: u32) -> FocusTrackerResult<Vec<u8>> {
    let size_i = size as isize;
    let size_f = size as f64;

    let bitmap_rep = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            std::ptr::null_mut(), // planes — let AppKit allocate
            size_i,               // pixelsWide
            size_i,               // pixelsHigh
            8,                    // bitsPerSample
            4,                    // samplesPerPixel (RGBA)
            true,                 // hasAlpha
            false,                // isPlanar
            NSCalibratedRGBColorSpace,
            0,                    // bytesPerRow  (0 = auto-calculate)
            0,                    // bitsPerPixel (0 = auto-calculate)
        )
    }
    .ok_or_else(|| FocusTrackerError::platform("failed to create target NSBitmapImageRep"))?;

    let context =
        NSGraphicsContext::graphicsContextWithBitmapImageRep(&bitmap_rep).ok_or_else(|| {
            FocusTrackerError::platform("failed to create NSGraphicsContext for icon rendering")
        })?;

    NSGraphicsContext::saveGraphicsState_class();
    NSGraphicsContext::setCurrentContext(Some(&context));

    let target_rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(size_f, size_f));
    ns_image.drawInRect(target_rect);

    NSGraphicsContext::restoreGraphicsState_class();

    let empty_props = NSDictionary::<NSString, AnyObject>::new();

    let png_data = unsafe {
        bitmap_rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &empty_props)
    }
    .ok_or_else(|| FocusTrackerError::platform("failed to encode rendered icon as PNG"))?;

    Ok(png_data.to_vec())
}
