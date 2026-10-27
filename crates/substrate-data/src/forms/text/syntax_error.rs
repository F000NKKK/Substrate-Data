//! [`SyntaxError`]: what doesn't parse, and where.

use std::fmt;

use super::LineCol;

/// A syntax error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxError {
    /// What's wrong.
    pub message: String,
    /// Its line and column.
    pub at: LineCol,
    /// Its byte offset.
    pub offset: usize,
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.at, self.message)
    }
}

impl std::error::Error for SyntaxError {}
