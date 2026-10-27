//! Editing text in place: [`patch`] changes a document to hold a
//! new value and keeps everything that didn't change as it was — comments,
//! key order, number spelling, layout. What changed is written by
//! [`format()`], the way editors write a value.

mod format;
mod rewrite;
#[cfg(test)]
mod tests;

pub use format::{format, same, to_json};
pub use rewrite::patch;
