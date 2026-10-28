//! [`ItemError`]: an item of a [`Stream`](crate::Stream) that cannot be read.

use std::fmt;

/// An item of a [`Stream`](crate::Stream) that cannot be read, found while
/// reading the stream's items (decoding a stream checks only its framing).
///
/// `index` is the item's place in the stream and `offset` the byte it starts
/// at, counted from the first item (the stream's count is not in it). When
/// `index` equals the stream's `len()` there is no item to read there: the
/// stream has bytes left after its last item, and `offset` is where they begin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemError {
    /// The item's place in the stream, from 0.
    pub index: usize,
    /// The byte the item starts at, from the first item.
    pub offset: usize,
}

impl fmt::Display for ItemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "stream item {} at byte {} cannot be read",
            self.index, self.offset
        )
    }
}

impl std::error::Error for ItemError {}
