use crate::MAX_FRAME_SIZE;
use crate::server::Frame;
use anyhow::{Context, Result, anyhow, bail};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64_STANDARD};
use image::{ImageBuffer, Rgba};
use resvg::render;
use specta_typescript::BigIntExportBehavior;
use std::process;
use tiny_skia::Pixmap;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tracing::debug;
use usvg::{Options, Tree};

pub fn convert_svg_to_rgba(svg: &str) -> Result<image::RgbaImage> {
    // Strip data URL prefix if present
    let b64 = svg
        .trim()
        .strip_prefix("data:image/svg+xml;base64,")
        .unwrap_or(svg);

    // Decode base64 SVG data
    let svg_bytes = BASE64_STANDARD
        .decode(b64)
        .map_err(|e| anyhow!("Failed to decode base64 SVG: {}", e))?;

    // Parse SVG with system fonts loaded
    let mut opt = Options::default();
    opt.fontdb_mut().load_system_fonts();

    let tree =
        Tree::from_data(&svg_bytes, &opt).map_err(|e| anyhow!("Failed to parse SVG: {}", e))?;

    // Get actual SVG dimensions
    let size = tree.size();
    let width = size.width().ceil() as u32;
    let height = size.height().ceil() as u32;

    // Create pixmap with correct dimensions
    let mut pixmap = Pixmap::new(width, height).ok_or_else(|| {
        anyhow!(
            "Failed to create pixmap with dimensions {}x{}",
            width,
            height
        )
    })?;

    // Render SVG to pixmap
    render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

    // Convert pixmap to image buffer
    let img = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, pixmap.data().to_vec())
        .ok_or_else(|| {
            anyhow!(
                "Failed to create image buffer from pixmap data ({}x{})",
                width,
                height
            )
        })?;

    Ok(img)
}

/// Find processes by name and return their PIDs
fn find_processes_by_name(process_name: &str) -> Result<Vec<u32>> {
    let mut pids = Vec::new();
    let current_pid = process::id();

    #[cfg(target_family = "unix")]
    {
        use std::process::Command;
        // On Unix-like systems, use pgrep to find processes by name
        let output = Command::new("pgrep").args(["-f", process_name]).output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    for line in output_str.lines() {
                        if let Ok(pid) = line.trim().parse::<u32>() {
                            // Don't include our own process
                            if pid != current_pid {
                                pids.push(pid);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                debug!("Failed to run pgrep: {}", e);
                // Fallback: try using ps
                let output = Command::new("ps").args(["aux"]).output();

                if let Ok(output) = output {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    for line in output_str.lines() {
                        if line.contains(process_name) {
                            let parts: Vec<&str> = line.split_whitespace().collect();
                            if parts.len() > 1
                                && let Ok(pid) = parts[1].parse::<u32>()
                                && pid != current_pid
                            {
                                pids.push(pid);
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_family = "windows")]
    {
        use std::process::Command;
        // On Windows, use tasklist to find processes by name
        let output = Command::new("tasklist")
            .args([
                "/FI",
                &format!("IMAGENAME eq {}.exe", process_name),
                "/FO",
                "CSV",
                "/NH",
            ])
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    for line in output_str.lines() {
                        if !line.trim().is_empty() {
                            // Parse CSV format: "process.exe","PID","Session Name","Session#","Mem Usage"
                            let parts: Vec<&str> = line.split(',').collect();
                            if parts.len() > 1 {
                                let pid_str = parts[1].trim_matches('"');
                                if let Ok(pid) = pid_str.parse::<u32>() {
                                    if pid != current_pid {
                                        pids.push(pid);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                debug!("Failed to run tasklist: {}", e);
            }
        }
    }

    Ok(pids)
}

/// Kill a process with the given PID
fn kill_process(pid: u32) -> Result<()> {
    #[cfg(target_family = "unix")]
    {
        use std::process::Command;
        // On Unix-like systems, we can use kill to terminate the process
        let status = Command::new("kill").args([&pid.to_string()]).status()?;

        if !status.success() {
            return Err(anyhow!("Failed to kill process {}", pid));
        }
    }

    #[cfg(target_family = "windows")]
    {
        use std::process::Command;
        // On Windows, we can use taskkill to terminate the process
        let status = Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .status()?;

        if !status.success() {
            return Err(anyhow!("Failed to kill process {}", pid));
        }
    }

    // Wait a moment for the process to terminate
    std::thread::sleep(std::time::Duration::from_millis(500));

    Ok(())
}

/// Ensure only one instance is running
pub fn ensure_single_instance() -> Result<()> {
    // Define the process name to search for
    let process_name = "euro-native-messaging";

    // Find any existing instances of this process
    let existing_pids = find_processes_by_name(process_name)?;

    // Kill all existing instances
    for pid in existing_pids {
        debug!("Found existing instance with PID {}. Killing it...", pid);
        if let Err(e) = kill_process(pid) {
            debug!("Failed to kill process {}: {}", pid, e);
            // Continue trying to kill other processes even if one fails
        }
    }

    // Register a shutdown handler for clean exit
    ctrlc::set_handler(move || {
        debug!("Received shutdown signal. Exiting...");
        process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    Ok(())
}

/// Generate TypeScript definitions using Specta
pub fn generate_typescript_definitions() -> Result<()> {
    use specta_typescript::Typescript;

    if let Err(e) = Typescript::default()
        .bigint(BigIntExportBehavior::Fail)
        .export_to(
            "packages/browser-shared/src/content/bindings.ts",
            &specta::export(),
        )
    {
        debug!("Failed to generate TypeScript definitions: {}", e);
    }

    Ok(())
}

pub async fn read_framed<R>(reader: &mut R) -> anyhow::Result<Option<Frame>>
where
    R: AsyncRead + Unpin,
{
    let mut len_buf = [0u8; 4];

    match reader.read_exact(&mut len_buf).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
            return Ok(None);
        }
        Err(e) => return Err(e).context("reading message length"),
    }

    let len = u32::from_le_bytes(len_buf) as usize;
    if len == 0 {
        // Chrome native messaging always sends valid JSON; empty is invalid.
        return Err(anyhow!("received empty frame (length = 0)"));
    }

    if len > MAX_FRAME_SIZE {
        bail!(
            "frame too large: {} bytes (limit {} bytes)",
            len,
            MAX_FRAME_SIZE
        );
    }

    let mut buf = vec![0u8; len];

    reader
        .read_exact(&mut buf)
        .await
        .context("reading message body")?;

    let frame: Frame = serde_json::from_slice(&buf).context("parsing Frame from JSON")?;

    Ok(Some(frame))
}

pub async fn write_framed<W>(writer: &mut W, frame: &Frame) -> anyhow::Result<()>
where
    W: AsyncWrite + Unpin,
{
    let json = serde_json::to_vec(frame).context("serializing Frame to JSON")?;
    let len = json.len();

    if len > u32::MAX as usize {
        bail!("frame too large: {} bytes (limit {} bytes)", len, u32::MAX);
    }

    let len = len as u32;
    let len_bytes = len.to_le_bytes();

    writer
        .write_all(&len_bytes)
        .await
        .context("writing message length")?;

    writer
        .write_all(&json)
        .await
        .context("writing message body")?;
    writer.flush().await.context("flushing stdout writer")?;

    Ok(())
}
