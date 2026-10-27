//! A schema's value in and out of the forms: values, bytes, text.

mod read;
mod write;

#[cfg(feature = "toml")]
pub use read::from_toml;
pub use read::{
    field_names, from_bytes_with, from_doc, from_doc_with, from_text, from_value, from_value_with,
    placed, read_both, read_bytes, read_ref_value, read_value,
};
pub use write::{to_bytes, to_text};
