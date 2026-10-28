//! The binary form and what carries it: frames, streams of items, byte
//! streams.

pub mod binary;
pub mod frame;
pub mod io;
pub(crate) mod item_error;
pub(crate) mod items;
pub(crate) mod stream;
pub(crate) mod stream_writer;
