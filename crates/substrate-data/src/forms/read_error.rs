//! [`ReadError`]: why a source file's bytes aren't a source.

use std::fmt;

use crate::binary::DecodeError;

/// Why a source file's bytes aren't a source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    /// Text that isn't UTF-8.
    NotUtf8(std::str::Utf8Error),
    /// The binary form, malformed.
    Binary(DecodeError),
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadError::NotUtf8(e) => write!(f, "not UTF-8 text: {e}"),
            ReadError::Binary(e) => write!(f, "malformed binary form: {e}"),
        }
    }
}

impl std::error::Error for ReadError {}
