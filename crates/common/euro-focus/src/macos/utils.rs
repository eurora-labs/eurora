use crate::{FocusedWindow, config::IconConfig, error::FerrousFocusResult};
use core_foundation::array::{CFArray, CFArrayRef};
use core_foundation::base::{CFType, TCFType};
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use objc2::ClassType;
use objc2::msg_send;
use objc2::rc::autoreleasepool;
use objc2::runtime::AnyObject;
use objc2_app_kit::{
    NSBitmapImageFileType, NSBitmapImageRep, NSCompositingOperation, NSImage, NSRunningApplication,
    NSWorkspace,
};
use objc2_foundation::{NSDictionary, NSPoint, NSRect, NSSize, NSString, ns_string};
use std::ffi::c_void;

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXUIElementCreateApplication(pid: i32) -> *mut AnyObject;
    fn AXUIElementCopyAttributeValue(
        element: *const AnyObject,
        attribute: *const AnyObject,
        value: *mut *mut AnyObject,
    ) -> i32;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(cf: *const c_void);
    fn CFStringGetLength(theString: *const c_void) -> isize;
    fn CFStringGetCString(
        theString: *const c_void,
        buffer: *mut i8,
        bufferSize: isize,
        encoding: u32,
    ) -> bool;
}

const K_CF_STRING_ENCODING_UTF8: u32 = 0x08000100;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGWindowListCopyWindowInfo(option: u32, relative_to_window: u32) -> CFArrayRef;
}

const K_AX_ERROR_SUCCESS: i32 = 0;
const K_AX_ERROR_APIDISABLED: i32 = -25211;
const K_CG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY: u32 = 1;
const K_CG_WINDOW_LIST_EXCLUDE_DESKTOP_ELEMENTS: u32 = 1 << 4;
const K_CG_NULL_WINDOW_ID: u32 = 0;

/// Get basic window info without icon (for change detection)
pub fn get_frontmost_window_basic_info() -> FerrousFocusResult<FocusedWindow> {
    autoreleasepool(|_pool| {
        let pid = get_frontmost_window_pid()?;

        let running_app = NSRunningApplication::runningApplicationWithProcessIdentifier(pid);

        let process_name = if let Some(ref app) = running_app {
            let name = app.localizedName();
            name.map(|n| n.to_string())
        } else {
            None
        };

        let window_title = get_window_title_via_accessibility(pid)?;

        Ok(FocusedWindow {
            process_id: Some(pid as u32),
            window_title,
            process_name,
            icon: None,
        })
    })
}

/// Fetch icon for the given process ID
pub fn fetch_icon_for_pid(
    pid: i32,
    icon_config: &IconConfig,
) -> FerrousFocusResult<Option<image::RgbaImage>> {
    autoreleasepool(|_pool| {
        let running_app = NSRunningApplication::runningApplicationWithProcessIdentifier(pid);
        if let Some(app) = running_app {
            get_app_icon(&app, icon_config)
        } else {
            Ok(None)
        }
    })
}

fn get_frontmost_window_pid() -> FerrousFocusResult<i32> {
    unsafe {
        // Get list of all on-screen windows, ordered by front-to-back
        let options =
            K_CG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY | K_CG_WINDOW_LIST_EXCLUDE_DESKTOP_ELEMENTS;
        let window_list_ref = CGWindowListCopyWindowInfo(options, K_CG_NULL_WINDOW_ID);

        if window_list_ref.is_null() {
            return Err(crate::error::FerrousFocusError::Platform(
                "Failed to get window list".to_string(),
            ));
        }

        let window_list: CFArray<CFDictionary> = CFArray::wrap_under_create_rule(window_list_ref);

        if window_list.is_empty() {
            return Err(crate::error::FerrousFocusError::Platform(
                "No windows found".to_string(),
            ));
        }

        let layer_key = CFString::from_static_string("kCGWindowLayer");
        let pid_key = CFString::from_static_string("kCGWindowOwnerPID");

        // Find the first window at layer 0 (normal application windows)
        for i in 0..window_list.len() {
            let window_info = window_list.get(i).ok_or_else(|| {
                crate::error::FerrousFocusError::Platform(format!("Failed to get window {}", i))
            })?;

            // Check window layer
            if let Some(layer_ptr) = window_info.find(layer_key.as_CFTypeRef() as *const _) {
                let layer_cftype = CFType::wrap_under_get_rule(layer_ptr.cast());
                if let Some(layer_number) = layer_cftype.downcast::<CFNumber>()
                    && let Some(layer) = layer_number.to_i32()
                {
                    // Skip non-zero layers (these are overlays, menus, etc.)
                    if layer != 0 {
                        continue;
                    }
                }
            }

            // Get the PID for this window
            let pid_value_ptr = window_info
                .find(pid_key.as_CFTypeRef() as *const _)
                .ok_or_else(|| {
                    crate::error::FerrousFocusError::Platform(
                        "Failed to get window owner PID".to_string(),
                    )
                })?;

            let pid_cftype = CFType::wrap_under_get_rule(pid_value_ptr.cast());
            let pid_number: CFNumber = pid_cftype.downcast().ok_or_else(|| {
                crate::error::FerrousFocusError::Platform(
                    "Failed to downcast PID to CFNumber".to_string(),
                )
            })?;
            let pid: i32 = pid_number.to_i32().ok_or_else(|| {
                crate::error::FerrousFocusError::Platform(
                    "Failed to convert PID to i32".to_string(),
                )
            })?;

            return Ok(pid);
        }

        Err(crate::error::FerrousFocusError::Platform(
            "No normal application window found".to_string(),
        ))
    }
}

fn get_window_title_via_accessibility(pid: i32) -> FerrousFocusResult<Option<String>> {
    let app_element = unsafe { AXUIElementCreateApplication(pid) };
    if app_element.is_null() {
        return Ok(None);
    }

    let focused_window_key = ns_string!("AXFocusedWindow");
    let mut focused_window: *mut AnyObject = std::ptr::null_mut();
    let result = unsafe {
        AXUIElementCopyAttributeValue(
            app_element,
            focused_window_key as *const NSString as *const AnyObject,
            &mut focused_window,
        )
    };

    // Release app_element - it follows the Create Rule
    unsafe { CFRelease(app_element as *const c_void) };

    if result == K_AX_ERROR_APIDISABLED {
        return Err(crate::error::FerrousFocusError::PermissionDenied);
    }

    if result != K_AX_ERROR_SUCCESS || focused_window.is_null() {
        return Ok(None);
    }

    let title_key = ns_string!("AXTitle");
    let mut title: *mut AnyObject = std::ptr::null_mut();
    let result = unsafe {
        AXUIElementCopyAttributeValue(
            focused_window,
            title_key as *const NSString as *const AnyObject,
            &mut title,
        )
    };

    // Release focused_window - it follows the Create Rule
    unsafe { CFRelease(focused_window as *const c_void) };

    if result != K_AX_ERROR_SUCCESS || title.is_null() {
        return Ok(None);
    }

    // Extract string from CFString and then release it
    // The title follows the Create Rule from AXUIElementCopyAttributeValue
    let title_str = unsafe { cfstring_to_string(title as *const c_void) };

    // Release title - it follows the Create Rule
    unsafe { CFRelease(title as *const c_void) };

    Ok(title_str)
}

/// Convert a CFString to a Rust String
///
/// # Safety
/// The caller must ensure the pointer is a valid CFString
unsafe fn cfstring_to_string(cf_string: *const c_void) -> Option<String> {
    if cf_string.is_null() {
        return None;
    }

    let length = unsafe { CFStringGetLength(cf_string) };
    if length <= 0 {
        return Some(String::new());
    }

    // UTF-8 can use up to 4 bytes per character, plus null terminator
    let buffer_size = (length * 4 + 1) as usize;
    let mut buffer: Vec<i8> = vec![0; buffer_size];

    let success = unsafe {
        CFStringGetCString(
            cf_string,
            buffer.as_mut_ptr(),
            buffer_size as isize,
            K_CF_STRING_ENCODING_UTF8,
        )
    };

    if success {
        // Find the null terminator and convert to String
        let c_str = unsafe { std::ffi::CStr::from_ptr(buffer.as_ptr()) };
        c_str.to_str().ok().map(|s| s.to_string())
    } else {
        None
    }
}

fn get_app_icon(
    app: &NSRunningApplication,
    icon_config: &IconConfig,
) -> FerrousFocusResult<Option<image::RgbaImage>> {
    let bundle_url = match app.bundleURL() {
        Some(url) => url,
        None => return Ok(None),
    };

    let path = match bundle_url.path() {
        Some(p) => p,
        None => return Ok(None),
    };

    let workspace = NSWorkspace::sharedWorkspace();
    let icon = workspace.iconForFile(&path);

    let rgba_image = nsimage_to_rgba(&icon, icon_config)?;
    Ok(Some(rgba_image))
}

fn nsimage_to_rgba(
    image: &NSImage,
    icon_config: &IconConfig,
) -> FerrousFocusResult<image::RgbaImage> {
    let icon_size = icon_config.get_size_or_default() as f64;

    let size = NSSize {
        width: icon_size,
        height: icon_size,
    };

    image.setSize(size);

    let rect = NSRect {
        origin: NSPoint { x: 0.0, y: 0.0 },
        size,
    };

    let bitmap_rep = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            msg_send![NSBitmapImageRep::class(), alloc],
            std::ptr::null_mut(),
            icon_size as isize,
            icon_size as isize,
            8,
            4,
            true,
            false,
            ns_string!("NSCalibratedRGBColorSpace"),
            0,
            0,
        )
    };

    if bitmap_rep.is_none() {
        return Err(crate::error::FerrousFocusError::Platform(
            "Failed to create bitmap representation".to_string(),
        ));
    }
    let bitmap_rep = bitmap_rep.unwrap();

    let ns_graphics_context_class = objc2::class!(NSGraphicsContext);
    let graphics_context: *mut AnyObject = unsafe {
        msg_send![
            ns_graphics_context_class,
            graphicsContextWithBitmapImageRep: &*bitmap_rep
        ]
    };

    unsafe {
        let _: () = msg_send![ns_graphics_context_class, saveGraphicsState];
        let _: () = msg_send![ns_graphics_context_class, setCurrentContext: graphics_context];
    }

    let from_rect = NSRect {
        origin: NSPoint { x: 0.0, y: 0.0 },
        size: NSSize {
            width: 0.0,
            height: 0.0,
        },
    };
    image.drawInRect_fromRect_operation_fraction(
        rect,
        from_rect,
        NSCompositingOperation::Copy,
        1.0,
    );

    unsafe {
        let _: () = msg_send![ns_graphics_context_class, restoreGraphicsState];
    }

    let empty_dict = NSDictionary::new();
    let png_data = unsafe {
        bitmap_rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &empty_dict)
    };

    if png_data.is_none() {
        return Err(crate::error::FerrousFocusError::Platform(
            "Failed to get PNG data from bitmap".to_string(),
        ));
    }
    let png_data = png_data.unwrap();

    let bytes = unsafe {
        let data_ptr: *const std::ffi::c_void = msg_send![&*png_data, bytes];
        std::slice::from_raw_parts(data_ptr as *const u8, png_data.len())
    };

    let rgba_image = image::load_from_memory(bytes)
        .map_err(|e| {
            crate::error::FerrousFocusError::Platform(format!(
                "Failed to load image from PNG data: {}",
                e
            ))
        })?
        .to_rgba8();

    Ok(rgba_image)
}
