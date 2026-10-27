//! The text syntax: JSON, plus `//` and `/* */` comments, trailing commas
//! and unquoted keys (the part of JSON5 editors already understand).
//!
//! [`parse`] keeps where every value is ([`Spans`]), so a schema error at
//! `entries[2].pages[0].item` can point at its line. Typed values stay
//! strings here — the schema decides what a string means.

mod comments;
mod doc;
mod line_col;
mod parser;
mod scan;
mod span;
mod spans;
mod syntax_error;
mod writer;

#[cfg(test)]
mod tests;

pub use comments::with_comments;
pub use doc::Doc;
pub use line_col::LineCol;
pub use parser::{MAX_DEPTH, parse, parse_value};
pub use span::Span;
pub use spans::Spans;
pub use syntax_error::SyntaxError;
pub(crate) use writer::float_string;
pub use writer::{to_string, to_string_pretty, write_string};
