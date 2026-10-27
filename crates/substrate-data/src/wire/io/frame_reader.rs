//! [`FrameReader`]: reads frames into one buffer it keeps.

use std::io::{self, Read};

use super::MAX_FRAME;
use super::framing::{decode_frame, read_len};
use crate::Schema;

/// Reads frames into a buffer it keeps, so a stream of messages costs no
/// allocation per frame once the buffer has grown to the biggest. The end
/// rules are those of [`super::read_frame_bytes`]: a clean end only between
/// frames; a stream cut inside a length or a frame is an error.
///
/// ```
/// use substrate_data::io::{FrameReader, write_frame};
///
/// let mut pipe = Vec::new();
/// write_frame(&mut pipe, &7_u32)?;
/// write_frame(&mut pipe, &8_u32)?;
/// let (mut r, mut frames) = (pipe.as_slice(), FrameReader::new());
/// assert_eq!(frames.read::<u32>(&mut r)?, Some(7));
/// assert_eq!(frames.read::<u32>(&mut r)?, Some(8));
/// assert_eq!(frames.read::<u32>(&mut r)?, None);
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Debug, Default)]
pub struct FrameReader {
    buf: Vec<u8>,
}

impl FrameReader {
    /// A reader with an empty buffer.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The next frame's bytes, valid until the next read; `None` where the
    /// stream ends between frames. The buffer grows as bytes arrive, never
    /// by a length the peer merely claims.
    ///
    /// # Errors
    ///
    /// The reader's I/O error; [`io::ErrorKind::UnexpectedEof`] when the
    /// stream ends inside a length or a frame; [`io::ErrorKind::InvalidData`]
    /// for a length over [`MAX_FRAME`].
    pub fn read_bytes(&mut self, r: &mut impl Read) -> io::Result<Option<&[u8]>> {
        let Some(len) = read_len(r)? else {
            return Ok(None);
        };
        let len = len as usize;
        if len > MAX_FRAME {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "frame too long"));
        }
        self.buf.clear();
        r.take(len as u64).read_to_end(&mut self.buf)?;
        if self.buf.len() < len {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        Ok(Some(&self.buf))
    }

    /// The next message; `None` where the stream ends between frames.
    ///
    /// # Errors
    ///
    /// The reader's I/O error, or [`io::ErrorKind::InvalidData`] for a frame
    /// that isn't a `T`.
    pub fn read<T: Schema>(&mut self, r: &mut impl Read) -> io::Result<Option<T>> {
        self.read_bytes(r)?.map(decode_frame).transpose()
    }
}
