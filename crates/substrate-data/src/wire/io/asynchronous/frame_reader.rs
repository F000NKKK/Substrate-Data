//! [`FrameReader`]: reads frames from an async stream into one buffer it keeps.

use std::io;

use tokio::io::{AsyncRead, AsyncReadExt};

use crate::Schema;
use crate::io::Len;
use crate::wire::io::framing::{body_len, decode_frame};

/// The least a body buffer grows by while bytes arrive.
const STEP: usize = 64 << 10;

/// Reads frames into a buffer it keeps, as [`crate::io::FrameReader`] does,
/// from an `AsyncRead`. The end rules are the same: a clean end only between
/// frames; a stream cut inside a length or a frame is
/// [`io::ErrorKind::UnexpectedEof`].
///
/// It is cancel safe: what has arrived of a frame stays here, so dropping a
/// read and calling again (on the same stream) loses nothing. The buffer
/// grows with the bytes that arrive, at most doubling, never by a length the
/// peer merely claims.
///
/// ```
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> std::io::Result<()> {
/// use substrate_data::io::{asynchronous::FrameReader, write_frame};
///
/// let mut pipe = Vec::new();
/// write_frame(&mut pipe, &7_u32)?;
/// let (mut r, mut frames) = (pipe.as_slice(), FrameReader::new());
/// assert_eq!(frames.read::<u32>(&mut r).await?, Some(7));
/// assert_eq!(frames.read::<u32>(&mut r).await?, None);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Default)]
pub struct FrameReader {
    header: [u8; size_of::<Len>()],
    /// How many bytes of `header` have arrived.
    header_len: usize,
    /// The body's length, once the header is whole.
    expect: Option<usize>,
    /// The body buffer; only `..filled` has arrived, the rest is spare room.
    body: Vec<u8>,
    filled: usize,
}

impl FrameReader {
    /// A reader with an empty buffer.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The next frame's bytes, valid until the next read; `None` where the
    /// stream ends between frames. Cancel safe.
    ///
    /// # Errors
    ///
    /// The reader's I/O error; [`io::ErrorKind::UnexpectedEof`] when the
    /// stream ends inside a length or a frame; [`io::ErrorKind::InvalidData`]
    /// for a length over [`crate::io::MAX_FRAME`].
    pub async fn read_bytes(
        &mut self,
        r: &mut (impl AsyncRead + Unpin),
    ) -> io::Result<Option<&[u8]>> {
        let len = match self.expect {
            Some(len) => len,
            None => match self.read_header(r).await? {
                Some(len) => len,
                None => return Ok(None),
            },
        };
        self.read_body(r, len).await?;
        self.header_len = 0;
        self.expect = None;
        self.filled = 0;
        Ok(Some(&self.body[..len]))
    }

    /// The next message; `None` where the stream ends between frames. Cancel
    /// safe up to the last `.await`: a frame that is whole is decoded at once.
    ///
    /// # Errors
    ///
    /// The reader's I/O error, or [`io::ErrorKind::InvalidData`] for a frame
    /// that isn't a `T`.
    pub async fn read<T: Schema>(
        &mut self,
        r: &mut (impl AsyncRead + Unpin),
    ) -> io::Result<Option<T>> {
        self.read_bytes(r).await?.map(decode_frame).transpose()
    }

    /// Like [`Self::read_bytes`], handing the frame over as its own `Vec`.
    pub(super) async fn read_owned(
        mut self,
        r: &mut (impl AsyncRead + Unpin),
    ) -> io::Result<Option<Vec<u8>>> {
        let Some(len) = self.read_bytes(r).await?.map(<[u8]>::len) else {
            return Ok(None);
        };
        self.body.truncate(len);
        Ok(Some(self.body))
    }

    /// Reads the length; `None` only when the stream ends before its first byte.
    async fn read_header(&mut self, r: &mut (impl AsyncRead + Unpin)) -> io::Result<Option<usize>> {
        while self.header_len < self.header.len() {
            let n = r.read(&mut self.header[self.header_len..]).await?;
            if n == 0 {
                return if self.header_len == 0 {
                    Ok(None)
                } else {
                    Err(io::ErrorKind::UnexpectedEof.into())
                };
            }
            self.header_len += n;
        }
        let len = body_len(Len::from_le_bytes(self.header))?;
        self.expect = Some(len);
        Ok(Some(len))
    }

    /// Reads the body's `len` bytes, growing the buffer only as they arrive.
    async fn read_body(&mut self, r: &mut (impl AsyncRead + Unpin), len: usize) -> io::Result<()> {
        while self.filled < len {
            let target = len.min(self.filled + STEP.max(self.filled));
            if self.body.len() < target {
                self.body.resize(target, 0);
            }
            let end = self.body.len().min(len);
            let n = r.read(&mut self.body[self.filled..end]).await?;
            if n == 0 {
                return Err(io::ErrorKind::UnexpectedEof.into());
            }
            self.filled += n;
        }
        Ok(())
    }
}
