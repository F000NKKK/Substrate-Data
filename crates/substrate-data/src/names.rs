//! The words the formats share, defined once: the typed value grammar's
//! mark and the keys a format's own structure uses. Problem codes are
//! [`crate::diagnostic::Problem`]; the words a shape is described by are
//! [`crate::schema::ShapeNode`]'s variants.

use std::fmt;

/// What starts a typed value: `@file:images/logo.png`.
pub const TYPED: char = '@';

/// Keys a format's own structure uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Word {
    /// The key naming a tagged enum's variant: `{ type: "estimate", … }`.
    Type,
    /// Where a tagged enum's variant keeps what it wraps when that isn't a struct.
    Value,
}

impl Word {
    /// As written.
    pub const fn as_str(self) -> &'static str {
        match self {
            Word::Type => "type",
            Word::Value => "value",
        }
    }
}

impl fmt::Display for Word {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
