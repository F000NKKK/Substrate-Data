//! Negative cases for what version 2 adds: shapes, records, the tags that
//! are an int or a string by themselves.

use substrate_data::binary::{MAGIC, VERSION, decode, decode_ref};
use substrate_data::{Bump, Value};

use crate::case::{Case, Fault};

const MAX_DEPTH: usize = 256;

/// A version 2 document: the header and `body`.
fn document(body: &[u8]) -> Vec<u8> {
    let mut out = MAGIC.to_vec();
    out.push(VERSION);
    out.extend_from_slice(body);
    out
}

/// A negative case: `body` (after the header) must fail with a message containing `says`.
fn fault(name: &str, why: &str, body: &[u8], kind: &'static str, says: &str) -> Case {
    let input = document(body);
    let owned = decode(&input).expect_err("malformed bytes");
    assert!(owned.message.contains(says), "{name}: {owned}");
    let arena = Bump::new();
    let borrowed = decode_ref(&input, &arena).expect_err("malformed bytes");
    assert_eq!(
        (&borrowed.message, borrowed.offset),
        (&owned.message, owned.offset),
        "{name}"
    );
    let code = substrate_data::read_bytes::<Value>(&input)
        .expect_err("malformed bytes")
        .first()
        .and_then(|d| d.code)
        .map(|c| c.to_string())
        .expect("a code");
    Case::new(name, why).fault(Fault::Binary {
        input,
        kind,
        offset: owned.offset,
        message: owned.message,
        code,
    })
}

/// A row: name, why, the bytes after the header, kind, what the message says.
type Row = (
    &'static str,
    &'static str,
    &'static [u8],
    &'static str,
    &'static str,
);

/// Tag 15 is a shaped map: a shape marker (even: a shape's number; odd: its
/// key count in the other bits, then the keys as strings), then a value per
/// key. Tag 16 is records: a count, a shape, then each record's values.
/// 130 is the int 2, `[3, b'a']` the new string `a`.
const ROWS: [Row; 18] = [
    (
        "binary_v2_shape_undefined",
        "A shaped map that names shape 0 before any shape is defined.",
        &[15, 0],
        "bad_shape_index",
        "isn't defined",
    ),
    (
        "binary_v2_shape_undefined_second",
        "The second map names shape 1 when only shape 0 exists.",
        &[6, 2, 15, 3, 3, b'a', 130, 15, 2, 130],
        "bad_shape_index",
        "isn't defined",
    ),
    (
        "binary_v2_shape_no_keys",
        "A shape that says it has no keys: a map with no keys is tag 7, count 0.",
        &[15, 1],
        "bad_length",
        "a shape with no keys",
    ),
    (
        "binary_v2_shape_keys_past_end",
        "A shape that says 5 keys with 2 bytes left.",
        &[15, 11, 3, b'a'],
        "bad_length",
        "past the end",
    ),
    (
        "binary_v2_shape_key_repeated_as_text",
        "A shape whose second key spells the first one's text again as a new string.",
        &[15, 5, 3, b'a', 3, b'a', 130, 130],
        "duplicate_key",
        "already set",
    ),
    (
        "binary_v2_shape_key_repeated_by_index",
        "A shape whose second key is the first one again, by its table index.",
        &[15, 5, 3, b'a', 0, 130, 130],
        "duplicate_key",
        "already set",
    ),
    (
        "binary_v2_shape_key_not_utf8",
        "A shape's key that is not UTF-8.",
        &[15, 3, 5, 0xff, 0xfe, 130],
        "bad_utf8",
        "UTF-8",
    ),
    (
        "binary_v2_shape_values_missing",
        "A shape of one key and no value after it: checked against the bytes left.",
        &[15, 3, 3, b'a'],
        "bad_length",
        "past the end",
    ),
    (
        "binary_v2_shape_values_cut",
        "A shape of two keys and one byte left for the values: more values than bytes.",
        &[15, 5, 3, b'a', 3, b'b', 130],
        "bad_length",
        "past the end",
    ),
    (
        "binary_v2_shape_value_cut",
        "A shape of two keys whose second value is a float with no bytes.",
        &[15, 5, 3, b'a', 3, b'b', 130, 4],
        "unexpected_end",
        "unexpected end",
    ),
    (
        "binary_v2_records_count_past_end",
        "Records that say 9 maps of one value each with none left.",
        &[16, 9, 3, 3, b'a'],
        "bad_length",
        "past the end",
    ),
    (
        "binary_v2_records_values_cut",
        "Records that say 2 maps of one value each with one value left.",
        &[16, 2, 3, 3, b'a', 130],
        "bad_length",
        "past the end",
    ),
    (
        "binary_v2_records_shape_undefined",
        "Records that name shape 0 before it is defined.",
        &[16, 1, 0],
        "bad_shape_index",
        "isn't defined",
    ),
    (
        "binary_v2_records_no_keys",
        "Records of a shape with no keys.",
        &[16, 1, 1],
        "bad_length",
        "a shape with no keys",
    ),
    (
        "binary_v2_string_tag_missing",
        "Tag 32 is the table string 0, with nothing in the table yet.",
        &[32],
        "bad_string_index",
        "isn't in the table",
    ),
    (
        "binary_v2_string_tag_after_end",
        "Tag 33 is table string 1 when only string 0 is in the table.",
        &[6, 2, 5, 3, b'a', 33],
        "bad_string_index",
        "isn't in the table",
    ),
    (
        "binary_v2_trailing_after_inline_int",
        "A tag that is an int by itself, then one more byte.",
        &[130, 0],
        "trailing_bytes",
        "bytes after",
    ),
    (
        "binary_v2_trailing_after_records",
        "Records followed by one more byte.",
        &[16, 1, 3, 3, b'a', 130, 0],
        "trailing_bytes",
        "bytes after",
    ),
];

/// Tags version 2 leaves free: 17 to 31 and 64 to 127 (a version 2 document may not use them yet).
fn unknown_tags() -> Vec<Case> {
    [17u8, 31, 64, 127]
        .iter()
        .map(|&t| {
            fault(
                &format!("binary_v2_unknown_tag_{t}"),
                "A tag byte version 2 does not use: 0..=16, 32..=63 and 128..=255 are tags.",
                &[t],
                "unknown_tag",
                "unknown tag",
            )
        })
        .collect()
}

/// `repeat` copies of `unit` after `first`, then an int.
fn nested(first: &[u8], unit: &[u8], repeat: usize) -> Vec<u8> {
    let mut body = first.to_vec();
    for _ in 0..repeat {
        body.extend_from_slice(unit);
    }
    body.push(130);
    body
}

fn depth() -> Vec<Case> {
    let mut out = vec![
        fault(
            "binary_v2_depth_257_shaped_maps",
            "Shaped maps nested 257 deep: more than MAX_DEPTH (256).",
            &nested(&[15, 3, 3, b'k'], &[15, 0], MAX_DEPTH),
            "too_deep",
            "nested deeper",
        ),
        fault(
            "binary_v2_depth_records",
            "Records count two levels each (the list and its maps): 129 of them are 258 deep.",
            &nested(&[16, 1, 3, 3, b'k'], &[16, 1, 0], 128),
            "too_deep",
            "nested deeper",
        ),
    ];
    for (name, body) in [
        (
            "binary_v2_depth_256_shaped_maps",
            nested(&[15, 3, 3, b'k'], &[15, 0], MAX_DEPTH - 1),
        ),
        (
            "binary_v2_depth_256_records",
            nested(&[16, 1, 3, 3, b'k'], &[16, 1, 0], MAX_DEPTH / 2 - 1),
        ),
    ] {
        let input = document(&body);
        let value = decode(&input).expect("256 levels are fine");
        assert_eq!(
            substrate_data::binary::encode(&value),
            input,
            "{name}: written back"
        );
        out.push(
            Case::new(
                name,
                "Nested exactly 256 deep (MAX_DEPTH) decodes; the innermost holds the int 2. No value is given (the manifest cannot nest that deep): it decodes, and writing what it decodes to gives the same bytes.",
            )
            .binary_v2(input),
        );
    }
    out
}

fn counts() -> Vec<Case> {
    let mut huge = vec![0xff; 8];
    huge.push(0x7f);
    let mut body = vec![16];
    body.extend_from_slice(&huge);
    vec![fault(
        "binary_v2_records_count_huge",
        "Records that say 2^63-1 maps: an error, no allocation.",
        &body,
        "bad_length",
        "past the end",
    )]
}

/// Every case of this module.
pub(crate) fn cases() -> Vec<Case> {
    let mut all: Vec<Case> = ROWS
        .iter()
        .map(|&(name, why, body, kind, says)| fault(name, why, body, kind, says))
        .collect();
    all.extend(unknown_tags());
    all.extend(depth());
    all.extend(counts());
    all
}
