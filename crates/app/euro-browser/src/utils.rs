use crate::{Frame, MAX_FRAME_SIZE};
use anyhow::{Context, Result, anyhow, bail};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub async fn read_framed<R>(reader: &mut R) -> Result<Option<Frame>>
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

pub async fn write_framed<W>(writer: &mut W, frame: &Frame) -> Result<()>
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
