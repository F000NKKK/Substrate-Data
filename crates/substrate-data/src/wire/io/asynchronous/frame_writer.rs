//! [`FrameWriter`]: sends messages to an async stream from buffers it keeps.

use std::io;

use tokio::io::{AsyncWrite, AsyncWriteExt};

use crate::Schema;

/// Sends messages as frames, as [`crate::io::FrameWriter`] does, to an
/// `AsyncWrite`: the same bytes, the same buffers kept, each frame (its
/// length, then its body) in one `write_all`, then a flush.
///
/// A send is not cancel safe: dropping it mid-frame leaves a partial frame on
/// the stream. After a dropped or failed send, close the stream.
///
/// ```
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> std::io::Result<()> {
/// use substrate_data::io::asynchronous::FrameWriter;
///
/// let mut pipe = Vec::new();
/// let mut sender = FrameWriter::new();
/// sender.send(&mut pipe, &"hello".to_string()).await?;
/// sender.send_packed(&mut pipe, &"again".to_string()).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Default)]
pub struct FrameWriter {
    inner: crate::io::FrameWriter,
}

impl FrameWriter {
    /// A writer with nothing grown yet: the first sends allocate.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sends `message` in the named binary form, and flushes.
    ///
    /// # Errors
    ///
    /// The writer's I/O error; [`io::ErrorKind::InvalidInput`] for a frame
    /// over [`crate::io::MAX_FRAME`] (nothing is written).
    pub async fn send<T: Schema>(
        &mut self,
        w: &mut (impl AsyncWrite + Unpin),
        message: &T,
    ) -> io::Result<()> {
        put(w, self.inner.named_frame(message)?).await
    }

    /// Sends `message` in the packed frame layout, and flushes.
    ///
    /// # Errors
    ///
    /// As [`Self::send`].
    pub async fn send_packed<T: Schema>(
        &mut self,
        w: &mut (impl AsyncWrite + Unpin),
        message: &T,
    ) -> io::Result<()> {
        put(w, self.inner.packed_frame(message)?).await
    }
}

/// `frame` written in one piece and flushed.
async fn put(w: &mut (impl AsyncWrite + Unpin), frame: &[u8]) -> io::Result<()> {
    w.write_all(frame).await?;
    w.flush().await
}
