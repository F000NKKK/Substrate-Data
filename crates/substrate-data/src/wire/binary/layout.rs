//! The header and the fixed sizes of the binary form.

/// What a binary file starts with.
pub const MAGIC: &[u8; 4] = b"SDAT";
/// The version of the form after the magic.
pub const VERSION: u8 = 1;

/// Nested deeper than this is malformed.
pub const MAX_DEPTH: usize = 256;

/// A stream's length: fixed width, so the writer fills it in after its items.
pub const STREAM_LEN_BYTES: usize = 4;

/// A string's varint: a new one's (odd, its length) or a reference to the
/// table (even, its index).
pub const NEW_STRING: u64 = 1;
