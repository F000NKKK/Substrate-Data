//! The sizes frames are laid out by.

/// The bytes of a `f64`.
pub(super) const F64_BYTES: usize = 8;
/// The bytes of a `f32`.
pub(super) const F32_BYTES: usize = 4;
/// A record's length, reserved before its fields are known: one byte, the
/// common case; a longer record moves its fields to make room.
pub(super) const LENGTH_RESERVED: usize = 1;
/// The biggest length one varint byte holds.
pub(super) const ONE_BYTE_MAX: usize = 0x7F;
/// Strings a reader keeps in itself before it needs the heap.
pub(super) const INLINE_STRINGS: usize = 16;
/// The most bytes a `u64` takes as a varint.
pub(super) const VARINT_MAX_BYTES: usize = 10;
