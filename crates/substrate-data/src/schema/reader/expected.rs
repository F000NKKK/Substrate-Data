//! [`Expected`]: what a field takes, as a reader's "expected …" problem
//! says it.

use std::fmt;

/// What a field takes, for the "expected …, found …" problem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Expected {
    /// `true` or `false`.
    Bool,
    /// An integer.
    Integer,
    /// An integer that isn't negative.
    Unsigned,
    /// A number.
    Number,
    /// A string.
    Str,
    /// Text (a string, or a typed value's spelling).
    Text,
    /// A list.
    List,
    /// An object.
    Object,
    /// `{ ok }` or `{ err }`.
    OkOrErr,
    /// A reference `@kind:id`.
    Reference,
    /// Bytes.
    Bytes,
    /// A name (an enum's variant).
    Name,
    /// A 128-bit id in hex.
    Uid,
    /// A list of exactly this many items.
    Items(usize),
}

impl fmt::Display for Expected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expected::Bool => f.write_str("true or false"),
            Expected::Integer => f.write_str("an integer"),
            Expected::Unsigned => f.write_str("an unsigned integer"),
            Expected::Number => f.write_str("a number"),
            Expected::Str => f.write_str("a string"),
            Expected::Text => f.write_str("text"),
            Expected::List => f.write_str("a list"),
            Expected::Object => f.write_str("an object"),
            Expected::OkOrErr => f.write_str("{ ok } or { err }"),
            Expected::Reference => f.write_str("a reference `@kind:id`"),
            Expected::Bytes => f.write_str("bytes"),
            Expected::Name => f.write_str("a name"),
            Expected::Uid => f.write_str("32 hex digits"),
            Expected::Items(n) => write!(f, "a list of {n}"),
        }
    }
}
