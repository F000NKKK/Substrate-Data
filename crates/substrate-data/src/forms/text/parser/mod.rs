//! The parser: text to a value, with the spans of its parts when asked, or
//! straight to a borrowed value ([`parse_ref`]) for a schema to read.

mod borrowed;
mod build;
mod entry;
mod number;
mod owned;
mod parser;
mod string;

pub use entry::{MAX_DEPTH, parse, parse_ref, parse_value};
