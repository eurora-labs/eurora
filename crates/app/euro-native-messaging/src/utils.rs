use crate::MAX_FRAME_SIZE;
use crate::server::Frame;
use anyhow::{Context, Result, anyhow, bail};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64_STANDARD};
use image::{ImageBuffer, Rgba};
use resvg::render;
use specta_typescript::BigIntExportBehavior;
use tiny_skia::Pixmap;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use usvg::{Options, Tree};

pub fn convert_svg_to_rgba(svg: &str) -> Result<image::RgbaImage> {
    let b64 = svg
        .trim()
        .strip_prefix("data:image/svg+xml;base64,")
        .unwrap_or(svg);

    let svg_bytes = BASE64_STANDARD
        .decode(b64)
        .map_err(|e| anyhow!("Failed to decode base64 SVG: {}", e))?;

    let mut opt = Options::default();
    opt.fontdb_mut().load_system_fonts();

    let tree =
        Tree::from_data(&svg_bytes, &opt).map_err(|e| anyhow!("Failed to parse SVG: {}", e))?;

    let size = tree.size();
    let width = size.width().ceil() as u32;
    let height = size.height().ceil() as u32;

    let mut pixmap = Pixmap::new(width, height).ok_or_else(|| {
        anyhow!(
            "Failed to create pixmap with dimensions {}x{}",
            width,
            height
        )
    })?;

    render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

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

pub fn generate_typescript_definitions() -> Result<()> {
    use specta_typescript::Typescript;

    if let Err(e) = Typescript::default()
        .bigint(BigIntExportBehavior::Fail)
        .export_to(
            "apps/browser/src/shared/content/bindings.ts",
            &specta::export(),
        )
    {
        tracing::debug!("Failed to generate TypeScript definitions: {}", e);
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
