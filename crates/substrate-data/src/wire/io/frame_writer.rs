//! [`FrameWriter`]: sends messages from buffers it keeps.

use std::io::{self, Write};

use super::framing::{Len, frame_len};
use crate::Schema;
use crate::binary::Writer;
use crate::frame::Packer;

/// Sends messages as frames without allocating per message: it keeps the
/// buffers a message is written into, so once they have grown to the
/// biggest message, a send only writes. Each frame (its length, then its
/// body) goes out in one `write_all`, then the writer is flushed.
///
/// - [`FrameWriter::send_packed`] sends the packed frame layout
///   ([`crate::frame`]): allocation-free once warm for messages whose
///   fields are plain values, strings, lists and records.
/// - [`FrameWriter::send`] sends the named binary form (`SDAT`): also
///   allocation-free once warm, except where a field writes through a
///   [`crate::Stream`], which builds its body apart.
///
/// ```
/// use substrate_data::io::{FrameWriter, read_frame};
///
/// let mut sender = FrameWriter::new();
/// let mut pipe = Vec::new();
/// sender.send(&mut pipe, &"hello".to_string())?;
/// sender.send(&mut pipe, &"again".to_string())?;
/// assert_eq!(read_frame::<String>(&mut pipe.as_slice())?.as_deref(), Some("hello"));
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Debug, Default)]
pub struct FrameWriter {
    /// The named form's writer: its buffer holds the length, then the body.
    named: Writer,
    packer: Packer,
    /// The packed form's length and body, side by side.
    packed: Vec<u8>,
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
    /// over [`super::MAX_FRAME`] (nothing is written).
    pub fn send<T: Schema>(&mut self, w: &mut impl Write, message: &T) -> io::Result<()> {
        self.named.restart(size_of::<Len>());
        message.encode(&mut self.named);
        put_frame(w, self.named.buffer_mut())
    }

    /// Sends `message` in the packed frame layout, and flushes.
    ///
    /// # Errors
    ///
    /// The writer's I/O error; [`io::ErrorKind::InvalidInput`] for a frame
    /// over [`super::MAX_FRAME`] (nothing is written).
    pub fn send_packed<T: Schema>(&mut self, w: &mut impl Write, message: &T) -> io::Result<()> {
        let packed = &mut self.packed;
        packed.clear();
        packed.extend_from_slice(&[0; size_of::<Len>()]);
        self.packer
            .with_frame(message, |frame| packed.extend_from_slice(frame));
        put_frame(w, packed)
    }
}

/// `buf` (room for a length, then a body) with its length filled in, written
/// in one piece and flushed.
fn put_frame(w: &mut impl Write, buf: &mut [u8]) -> io::Result<()> {
    let (len, body) = buf.split_at_mut(size_of::<Len>());
    len.copy_from_slice(&frame_len(body.len())?.to_le_bytes());
    w.write_all(buf)?;
    w.flush()
}
