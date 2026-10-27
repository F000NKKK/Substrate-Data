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

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "byte {}: {}", self.offset, self.message)
    }
}

impl std::error::Error for DecodeError {}
