//! The binary form — self-describing, compact: what a build compiles
//! content to, what saves and caches are kept in, what a reader that
//! doesn't know the schema can still walk. `docs/format.md` ("Binary form")
//! has the layout: a header (`SDAT`, a version byte), then the root value,
//! a tag byte and its payload. A string is written where it's first used
//! and referred to after that, so repeated keys and ids cost one varint
//! each and the writer never goes back over what it wrote.
//!
//! Nothing is aligned, and decoding checks every length against the
//! input, so malformed bytes are an error, never a panic or a huge
//! allocation. A [`crate::Schema`] writes itself here straight
//! ([`crate::Schema::encode`]) and reads itself back the same way
//! ([`crate::Schema::decode`]) — no value tree on the way.

mod codec;
mod decode_error;
mod decoder;
mod decoder_ref;
mod decoder_straight;
pub(crate) mod interner;
mod layout;
pub mod tag;
#[cfg(test)]
mod tests;
mod varint;
pub(crate) mod writer;

pub use codec::{decode, decode_ref, encode};
pub use decode_error::DecodeError;
pub use decoder::Decoder;
pub use layout::{MAGIC, MAX_DEPTH, VERSION};
pub use layout::{NEW_STRING, STREAM_LEN_BYTES};
pub(crate) use varint::{
    VARINT_BITS, VARINT_LAST_SHIFT, VARINT_MAX_SHIFT, VARINT_MORE, VARINT_SHIFT, read_varint,
    unzigzag, varint, zigzag,
};
pub use writer::Writer;
