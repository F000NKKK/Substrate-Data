//! UTF-8 validation at the three places bytes become text (source files,
//! the binary form, frames): the same answers and the same errors whether
//! or not the `simd-utf8` feature is on.

use substrate_data::binary::{decode, encode};
use substrate_data::frame::{read_frame, to_frame};
use substrate_data::source::{self, ReadError};
use substrate_data::{Schema, Value};

#[derive(Schema, Debug, Clone, PartialEq)]
struct Note {
    body: String,
}

/// A long string with multibyte characters at every alignment.
fn long_text() -> String {
    "ab naïve — 日本語 😀 ".repeat(40)
}

/// `bytes` with the byte at `at` replaced.
fn with_byte(bytes: &[u8], at: usize, byte: u8) -> Vec<u8> {
    let mut copy = bytes.to_vec();
    copy[at] = byte;
    copy
}

#[test]
fn a_source_reports_the_standard_utf8_error_at_every_position() {
    let text = long_text();
    assert_eq!(source::text(text.as_bytes()).unwrap(), text);
    for at in 0..text.len() {
        let bad = with_byte(text.as_bytes(), at, 0xff);
        let expected = std::str::from_utf8(&bad).unwrap_err();
        let Err(ReadError::NotUtf8(got)) = source::text(&bad) else {
            panic!("an 0xff byte at {at} should be a UTF-8 error");
        };
        assert_eq!(got, expected, "at {at}");
    }
}

#[test]
fn a_source_cut_inside_a_character_is_an_error() {
    let text = long_text();
    for cut in 1..text.len() {
        let cut_text = &text.as_bytes()[..cut];
        assert_eq!(
            source::text(cut_text).is_ok(),
            std::str::from_utf8(cut_text).is_ok(),
            "cut at {cut}"
        );
    }
}

#[test]
fn the_binary_form_rejects_an_invalid_string_and_keeps_a_valid_one() {
    let text = long_text();
    let bytes = encode(&Value::from(text.as_str()));
    assert_eq!(decode(&bytes).unwrap(), Value::from(text.as_str()));
    let body = bytes.len() - text.len();
    for at in (body..bytes.len()).step_by(5) {
        let bad = with_byte(&bytes, at, 0xff);
        let err = decode(&bad).unwrap_err();
        assert_eq!(err.message, "a string isn't UTF-8", "at {at}");
        assert_eq!(err.offset, body, "at {at}");
    }
}

#[test]
fn a_frame_rejects_an_invalid_string_and_keeps_a_valid_one() {
    let note = Note { body: long_text() };
    let frame = to_frame(&note);
    assert_eq!(read_frame::<Note>(&frame), Some(note.clone()));
    let body = frame
        .windows(5)
        .position(|w| w == "naïv".as_bytes())
        .unwrap();
    for at in body..body + 60 {
        let bad = with_byte(&frame, at, 0xff);
        assert_eq!(read_frame::<Note>(&bad), None, "at {at}");
    }
}
