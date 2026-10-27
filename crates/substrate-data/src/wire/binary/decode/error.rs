//! [`DecodeError`]: malformed binary data, and where.

use std::fmt;

/// Malformed binary data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeError {
    /// What's wrong.
    pub message: String,
    /// The byte it was found at.
    pub offset: usize,
}

impl DecodeError {
    /// A map's key that was already set, found at `offset`.
    pub(super) fn duplicate_key(key: &str, offset: usize) -> Self {
        DecodeError {
            message: format!("`{key}` is already set in this map"),
            offset,
        }
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "byte {}: {}", self.offset, self.message)
    }
}

impl std::error::Error for DecodeError {}
