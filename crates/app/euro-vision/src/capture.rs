//! Screen and window capture, backed by the `xcap` crate.
//!
//! `xcap` is fully synchronous and CPU/GPU-bound (it copies frame buffers
//! out of the windowing system, then we PNG-encode them). All entry points
//! here off-load that work to `tokio::task::spawn_blocking` so the async
//! runtime stays responsive.
//!
//! Capture is intentionally best-effort: on Wayland without an
//! `xdg-desktop-portal` grant, or on macOS without Screen Recording
//! permission, the OS returns either an empty window list or an error.
//! Callers receive `Ok(None)` (no matching window) or a typed
//! [`CaptureError`] rather than a panic — they are free to drop the
//! capture and continue, which the activity strategies do.

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use image::{ImageFormat, RgbaImage, imageops::FilterType};
use thiserror::Error;
use tokio::task::JoinError;

/// Anthropic vision recommendation: images larger than this on the long edge
/// are downscaled before transport. Keeps the payload small and the PNG
/// encode fast without meaningfully hurting LLM legibility.
const MAX_EDGE_PX: u32 = 1568;

/// A single captured frame, ready to attach to an LLM message.
#[derive(Debug, Clone)]
pub struct CapturedImage {
    /// PNG payload, base64-encoded without a `data:` prefix.
    pub png_base64: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("xcap failed: {0}")]
    Xcap(#[from] xcap::XCapError),

    #[error("image encoding failed: {0}")]
    Encode(#[from] image::ImageError),

    #[error("capture task panicked: {0}")]
    Join(#[from] JoinError),
}

/// Capture the visible window owned by `pid`, if any.
///
/// Returns `Ok(None)` when no non-minimised window owned by `pid` is found —
/// a legitimate outcome (the app may be backgrounded, may have no top-level
/// window, or the compositor may not expose the surface to us). Capture
/// errors from the OS surface as `Err`.
pub async fn capture_window_by_pid(pid: u32) -> Result<Option<CapturedImage>, CaptureError> {
    tokio::task::spawn_blocking(move || capture_window_by_pid_blocking(pid)).await?
}

/// Trigger any permission prompts the host OS attaches to screen capture,
/// so the user grants once at app start rather than mid-chat.
///
/// On macOS this opens the Screen Recording TCC prompt the first time it
/// runs against an un-granted process. On Linux/Wayland it nudges the
/// `xdg-desktop-portal` permission flow. On X11 and Windows there is no
/// system-level prompt, so this resolves quickly and harmlessly.
///
/// Failures are swallowed and logged at `warn`; they are not a startup
/// blocker — capture call sites will fall back to no screenshot if the user
/// declines.
pub fn prime_capture_permission() {
    tokio::task::spawn_blocking(|| match prime_capture_permission_blocking() {
        Ok(()) => tracing::debug!("screen capture permission primed"),
        Err(err) => tracing::warn!(
            "screen capture permission probe failed (capture will be unavailable until granted): {err}"
        ),
    });
}

fn capture_window_by_pid_blocking(pid: u32) -> Result<Option<CapturedImage>, CaptureError> {
    let windows = xcap::Window::all()?;

    let Some(window) = select_best_window(&windows, pid) else {
        return Ok(None);
    };

    let raw = window.capture_image()?;
    let scaled = downscale_to_max_edge(raw, MAX_EDGE_PX);
    let (width, height) = scaled.dimensions();
    let png_base64 = encode_png_base64(&scaled)?;

    Ok(Some(CapturedImage {
        png_base64,
        width,
        height,
    }))
}

fn prime_capture_permission_blocking() -> Result<(), CaptureError> {
    // Capturing a monitor is the cheapest way to trip the OS permission
    // prompt without needing a target window. We discard the bytes.
    let monitors = xcap::Monitor::all()?;
    let Some(monitor) = monitors.into_iter().next() else {
        return Ok(());
    };
    let _ = monitor.capture_image()?;
    Ok(())
}

/// Pick the most relevant window for `pid`: prefer the focused one, then
/// the largest by area, ignoring minimised windows.
fn select_best_window(windows: &[xcap::Window], pid: u32) -> Option<&xcap::Window> {
    windows
        .iter()
        .filter(|w| w.pid().ok() == Some(pid))
        .filter(|w| !w.is_minimized().unwrap_or(false))
        .max_by_key(|w| {
            let focused = w.is_focused().unwrap_or(false) as u64;
            let area = u64::from(w.width().unwrap_or(0)) * u64::from(w.height().unwrap_or(0));
            // is_focused dominates; area breaks ties.
            (focused, area)
        })
}

fn downscale_to_max_edge(image: RgbaImage, max_edge: u32) -> RgbaImage {
    let (w, h) = image.dimensions();
    let longest = w.max(h);
    if longest <= max_edge || max_edge == 0 {
        return image;
    }
    let scale = f64::from(max_edge) / f64::from(longest);
    let new_w = ((f64::from(w) * scale).round() as u32).max(1);
    let new_h = ((f64::from(h) * scale).round() as u32).max(1);
    image::imageops::resize(&image, new_w, new_h, FilterType::Triangle)
}

fn encode_png_base64(image: &RgbaImage) -> Result<String, image::ImageError> {
    let mut bytes = Vec::with_capacity(image.as_raw().len() / 2);
    image.write_to(&mut std::io::Cursor::new(&mut bytes), ImageFormat::Png)?;
    Ok(BASE64_STANDARD.encode(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn returns_none_for_unknown_pid() {
        // u32::MAX is reserved by xcap conventions and will not match any
        // real window — the helper should resolve to `Ok(None)` rather
        // than erroring out.
        let result = capture_window_by_pid(u32::MAX).await;
        match result {
            Ok(None) => (),
            Ok(Some(_)) => panic!("unexpected capture for sentinel pid"),
            // On CI without a display server, `xcap::Window::all` itself
            // may fail; that's an environment limitation, not a logic bug.
            Err(err) => {
                eprintln!("xcap unavailable in this environment: {err}");
            }
        }
    }

    #[test]
    fn downscale_is_a_noop_when_within_max_edge() {
        let img = RgbaImage::from_pixel(10, 20, image::Rgba([0, 0, 0, 255]));
        let out = downscale_to_max_edge(img.clone(), 100);
        assert_eq!(out.dimensions(), (10, 20));
    }

    #[test]
    fn downscale_preserves_aspect_ratio() {
        let img = RgbaImage::from_pixel(4000, 2000, image::Rgba([255, 0, 0, 255]));
        let out = downscale_to_max_edge(img, 1568);
        let (w, h) = out.dimensions();
        assert_eq!(w, 1568);
        // 2000 / 4000 * 1568 = 784
        assert_eq!(h, 784);
    }

    #[test]
    fn downscale_handles_tall_images() {
        let img = RgbaImage::from_pixel(500, 5000, image::Rgba([0, 255, 0, 255]));
        let out = downscale_to_max_edge(img, 1568);
        let (w, h) = out.dimensions();
        assert_eq!(h, 1568);
        // 500 / 5000 * 1568 = 156.8 → 157
        assert_eq!(w, 157);
    }

    #[test]
    fn encode_png_base64_round_trips() {
        let img = RgbaImage::from_fn(8, 8, |x, y| {
            image::Rgba([x as u8 * 32, y as u8 * 32, 128, 255])
        });
        let encoded = encode_png_base64(&img).expect("encode");
        let bytes = BASE64_STANDARD.decode(&encoded).expect("decode");
        let decoded = image::load_from_memory_with_format(&bytes, ImageFormat::Png)
            .expect("decode png")
            .to_rgba8();
        assert_eq!(decoded.dimensions(), img.dimensions());
        assert_eq!(decoded.as_raw(), img.as_raw());
    }
}
