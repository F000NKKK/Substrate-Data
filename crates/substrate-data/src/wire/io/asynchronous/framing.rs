//! Frames written and read with their length in front, over async streams.

use std::io;

use tokio::io::{AsyncRead, AsyncWrite};

use super::{FrameReader, FrameWriter};
use crate::Schema;
use crate::wire::io::framing::frame_len;

/// Writes `message` as one frame, and flushes. Not cancel safe.
///
/// # Errors
///
/// The writer's I/O error.
pub async fn write_frame<T: Schema>(
    w: &mut (impl AsyncWrite + Unpin),
    message: &T,
) -> io::Result<()> {
    FrameWriter::new().send(w, message).await
}

/// Writes `bytes` (a message's binary form) as one frame, and flushes. Not
/// cancel safe.
///
/// # Errors
///
/// The writer's I/O error; [`io::ErrorKind::InvalidInput`] for a frame over
/// [`crate::io::MAX_FRAME`].
pub async fn write_frame_bytes(w: &mut (impl AsyncWrite + Unpin), bytes: &[u8]) -> io::Result<()> {
    use tokio::io::AsyncWriteExt;

    let len = frame_len(bytes.len())?;
    let mut frame = Vec::with_capacity(size_of::<crate::io::Len>() + bytes.len());
    frame.extend_from_slice(&len.to_le_bytes());
    frame.extend_from_slice(bytes);
    w.write_all(&frame).await?;
    w.flush().await
}

/// The next frame's bytes; `None` where the stream ends between frames. Not
/// cancel safe: use a [`FrameReader`] where the read can be dropped.
///
/// # Errors
///
/// The reader's I/O error; [`io::ErrorKind::UnexpectedEof`] when the stream
/// ends inside a length or a frame; [`io::ErrorKind::InvalidData`] for a
/// length over [`crate::io::MAX_FRAME`].
pub async fn read_frame_bytes(r: &mut (impl AsyncRead + Unpin)) -> io::Result<Option<Vec<u8>>> {
    FrameReader::new().read_owned(r).await
}

/// The next message; `None` where the stream ends between frames. Not cancel
/// safe.
///
/// # Errors
///
/// The reader's I/O error, or [`io::ErrorKind::InvalidData`] for a frame that
/// isn't a `T`.
pub async fn read_frame<T: Schema>(r: &mut (impl AsyncRead + Unpin)) -> io::Result<Option<T>> {
    FrameReader::new().read(r).await
}
