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
    let len = frame_len(bytes.len())?;
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
/// The reader's I/O error; [`io::ErrorKind::UnexpectedEof`] when the stream
/// ends inside a length or a frame; [`io::ErrorKind::InvalidData`] for a
/// length over [`MAX_FRAME`].
pub fn read_frame_bytes(r: &mut impl Read) -> io::Result<Option<Vec<u8>>> {
    let Some(len) = read_len(r)? else {
        return Ok(None);
    };
    let len = body_len(len)?;
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
    decode_frame(&bytes).map(Some)
}

/// A length read off the wire as a body length; [`io::ErrorKind::InvalidData`]
/// over [`MAX_FRAME`], before anything is allocated for it.
pub(super) fn body_len(len: Len) -> io::Result<usize> {
    let len = len as usize;
    if len > MAX_FRAME {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "frame too long"));
    }
    Ok(len)
}

/// A frame body's length as written; [`io::ErrorKind::InvalidInput`] over [`MAX_FRAME`].
pub(super) fn frame_len(len: usize) -> io::Result<Len> {
    Len::try_from(len)
        .ok()
        .filter(|_| len <= MAX_FRAME)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "frame too long"))
}

/// A frame body read as a `T`; [`io::ErrorKind::InvalidData`] when it isn't one.
pub(super) fn decode_frame<T: Schema>(bytes: &[u8]) -> io::Result<T> {
    crate::read_bytes(bytes)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, crate::Errors(e)))
}

/// A frame's length; `None` only when the stream ends before its first byte
/// (between frames), an error when it ends inside the length.
pub(super) fn read_len(r: &mut impl Read) -> io::Result<Option<Len>> {
    let mut len = [0; size_of::<Len>()];
    loop {
        match r.read(&mut len[..1]) {
            Ok(0) => return Ok(None),
            Ok(_) => break,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    r.read_exact(&mut len[1..])?;
    Ok(Some(Len::from_le_bytes(len)))
}
