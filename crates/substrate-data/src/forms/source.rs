//! A source file as it's kept and as it's read: on disk the binary form
//! (what an editor may save), to people its text — formatted as editors
//! write it, so a line in an error is the line the editor shows. A file
//! still in text is read as it is.

pub use super::read_error::ReadError;
use crate::binary::{self, MAGIC};
use crate::text::SyntaxError;

/// Whether `bytes` are the binary form.
#[must_use]
pub fn is_binary(bytes: &[u8]) -> bool {
    bytes.starts_with(MAGIC)
}

/// A source file's text: its binary form written out, or its text as it is.
///
/// # Errors
///
/// [`ReadError`] when the text isn't UTF-8 or the binary form is malformed.
pub fn text(bytes: &[u8]) -> Result<String, ReadError> {
    if !is_binary(bytes) {
        return crate::utf8::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(ReadError::NotUtf8);
    }
    let value = binary::decode(bytes).map_err(ReadError::Binary)?;
    Ok(format!("{}\n", crate::patch::format(&value, 0)))
}

/// Text as a source file keeps it: binary, its comments kept as notes
/// ([`crate::text::with_comments`]).
///
/// # Errors
///
/// [`SyntaxError`] when the text doesn't parse.
pub fn binary(text: &str) -> Result<Vec<u8>, SyntaxError> {
    crate::text::with_comments(text).map(|v| binary::encode(&v))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_source_round_trips_through_its_binary_form() {
        let text = "{\n  // Long.\n  pages: 3,\n  cover: \"@file:cover.png\"\n}";
        let bytes = binary(text).unwrap();
        assert!(is_binary(&bytes));
        let shown = super::text(&bytes).unwrap();
        assert!(
            shown.contains("$pages") && shown.contains("@file:cover.png"),
            "{shown}"
        );
        assert_eq!(
            binary(&shown).unwrap(),
            bytes,
            "its text is the same file again"
        );
        assert_eq!(
            super::text(text.as_bytes()).unwrap(),
            text,
            "text stays text"
        );
    }
}
