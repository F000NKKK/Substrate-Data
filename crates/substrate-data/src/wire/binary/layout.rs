//! The header and the fixed sizes of the binary form.

/// What a binary file starts with.
pub const MAGIC: &[u8; 4] = b"SDAT";
/// The version of the form after the magic that every writer writes.
pub const VERSION: u8 = 2;

/// The first version, still read and never written: a v1 document is a v2
/// one that uses only the tags `0..=14` (the reader checks that it does).
pub const LEGACY_VERSION: u8 = 1;

/// The smallest int a tag of its own holds ([`crate::binary::tag::INT_INLINE`]).
pub const INLINE_INT_MIN: i64 = -16;

/// The largest int a tag of its own holds.
pub const INLINE_INT_MAX: i64 = 111;

/// How many table strings a tag of their own can name ([`crate::binary::tag::STR_INDEX`]).
pub const STR_INDEX_COUNT: u8 = 32;

/// Nested deeper than this is malformed.
pub const MAX_DEPTH: usize = 256;

/// A stream's length: fixed width, so the writer fills it in after its items.
pub const STREAM_LEN_BYTES: usize = 4;

/// A string's varint: a new one's (odd, its length) or a reference to the
/// table (even, its index).
pub const NEW_STRING: u64 = 1;
