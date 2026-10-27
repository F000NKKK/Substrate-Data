//! Messages over a byte stream — a socket, a pipe: each message a frame,
//! its length (a little-endian `u32`) and then its binary form. What
//! programs say to each other (a program and its editor, a tool's service).

#[cfg(feature = "tokio")]
pub mod asynchronous;
mod frame_reader;
mod frame_writer;
mod framing;
#[cfg(test)]
mod tests;

pub use frame_reader::FrameReader;
pub use frame_writer::FrameWriter;
pub use framing::{Len, MAX_FRAME, read_frame, read_frame_bytes, write_frame, write_frame_bytes};
