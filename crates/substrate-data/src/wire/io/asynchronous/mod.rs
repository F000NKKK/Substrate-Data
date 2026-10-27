//! Frames over `tokio` byte streams (feature `tokio`): the wire format, the
//! [`MAX_FRAME`](super::MAX_FRAME) limit and the end rules of the blocking
//! [`super`] functions, read and written with `.await`.
//!
//! Async is for genuine I/O only: these functions wait on a socket or a
//! pipe. Encoding and decoding stay plain synchronous code, so a message
//! held in memory never needs `async`. A stream is anything that is
//! `AsyncRead`/`AsyncWrite` and `Unpin` (a `TcpStream`, a `tokio::io::duplex`
//! half, a `&[u8]`, a `Vec<u8>`); wrap an `!Unpin` one in `Box::pin`.
//!
//! # Cancellation
//!
//! - [`FrameReader::read_bytes`] and [`FrameReader::read`] are cancel safe:
//!   the bytes read so far stay in the reader, and the next call carries on
//!   from them, so a `select!` or a timeout that drops the future loses no
//!   data. The same reader must go on with the same stream.
//! - The free `read_*` functions keep nothing between calls: dropping one
//!   mid-frame loses the bytes it read. Use a [`FrameReader`] where reads can
//!   be cancelled.
//! - Every write is not cancel safe: dropping a write mid-frame leaves a
//!   partial frame on the stream, which the peer cannot tell from a cut
//!   connection. After a dropped or failed write, close the stream.
//!
//! ```
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> std::io::Result<()> {
//! use substrate_data::io::asynchronous::{FrameReader, FrameWriter};
//!
//! let (mut near, mut far) = tokio::io::duplex(64);
//! let mut writer = FrameWriter::new();
//! writer.send(&mut near, &"hello".to_string()).await?;
//! drop(near);
//! let mut reader = FrameReader::new();
//! assert_eq!(reader.read::<String>(&mut far).await?.as_deref(), Some("hello"));
//! assert_eq!(reader.read::<String>(&mut far).await?, None);
//! # Ok(())
//! # }
//! ```

mod frame_reader;
mod frame_writer;
mod framing;

pub use frame_reader::FrameReader;
pub use frame_writer::FrameWriter;
pub use framing::{read_frame, read_frame_bytes, write_frame, write_frame_bytes};
