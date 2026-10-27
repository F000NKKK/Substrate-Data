//! The parser: text to a value, with the spans of its parts when asked, or
//! straight to a borrowed value ([`parse_ref`]) for a schema to read.

mod borrowed;
mod build;
mod entry;
mod keys;
mod number;
mod owned;
#[expect(clippy::module_inception, reason = "the module's one type")]
mod parser;
mod string;

pub use entry::{MAX_DEPTH, parse, parse_ref, parse_value};
