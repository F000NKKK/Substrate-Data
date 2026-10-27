//! Frames written and read with their length in front.

use std::io::{self, Read, Write};

use crate::Schema;

/// How a frame's length is written.
pub type Len = u32;
/// The longest frame read: anything longer is a broken or hostile peer.
pub const MAX_FRAME: usize = 256 << 20;

/// Writes `message` as one frame, and flushes.
///
/// # Errors
///
/// The writer's I/O error.
pub fn write_frame<T: Schema>(w: &mut impl Write, message: &T) -> io::Result<()> {
    write_frame_bytes(w, &crate::to_bytes(message))
}

/// Writes `bytes` (a message's binary form) as one frame, and flushes.
///
/// # Errors
///
/// The writer's I/O error; [`io::ErrorKind::InvalidInput`] for a frame over [`MAX_FRAME`].
pub fn write_frame_bytes(w: &mut impl Write, bytes: &[u8]) -> io::Result<()> {
    let len = Len::try_from(bytes.len())
        .ok()
        .filter(|_| bytes.len() <= MAX_FRAME)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "frame too long"))?;
    // One write: a reader never sees a length without its body behind it.
    let mut frame = Vec::with_capacity(size_of::<Len>() + bytes.len());
    frame.extend_from_slice(&len.to_le_bytes());
    frame.extend_from_slice(bytes);
    w.write_all(&frame)?;
    w.flush()
}

/// The next frame's bytes; `None` where the stream ends between frames.
///
/// # Errors
///
/// The reader's I/O error; [`io::ErrorKind::InvalidData`] for a length over [`MAX_FRAME`].
pub fn read_frame_bytes(r: &mut impl Read) -> io::Result<Option<Vec<u8>>> {
    let mut len = [0; size_of::<Len>()];
    match r.read_exact(&mut len) {
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        other => other?,
    }
    let len = Len::from_le_bytes(len) as usize;
    if len > MAX_FRAME {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "frame too long"));
    }
    let mut bytes = vec![0; len];
    r.read_exact(&mut bytes)?;
    Ok(Some(bytes))
}

/// The next message; `None` where the stream ends between frames.
///
/// # Errors
///
/// The reader's I/O error, or [`io::ErrorKind::InvalidData`] for a frame that isn't a `T`.
pub fn read_frame<T: Schema>(r: &mut impl Read) -> io::Result<Option<T>> {
    let Some(bytes) = read_frame_bytes(r)? else {
        return Ok(None);
    };
    crate::read_bytes(&bytes)
        .map(Some)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, crate::Errors(e)))
}
