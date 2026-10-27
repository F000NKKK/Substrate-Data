//! Negative cases for the binary form.

use substrate_data::binary::{MAGIC, VERSION, decode, decode_ref};
use substrate_data::{Bump, Value};

use crate::case::{Case, Fault};

const MAX_DEPTH: usize = 256;

/// The header and `body`.
fn document(body: &[u8]) -> Vec<u8> {
    let mut out = MAGIC.to_vec();
    out.push(VERSION);
    out.extend_from_slice(body);
    out
}

/// A varint of `n` continuing bytes, then `last`.
fn cont(n: usize, last: u8) -> Vec<u8> {
    let mut out = vec![0x80; n];
    out.push(last);
    out
}

/// A negative case: `input` must fail with a message containing `says`.
fn fault(name: &str, why: &str, input: Vec<u8>, kind: &'static str, says: &str) -> Case {
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

/// A fault with a literal body: name, why, the bytes after the header, kind, what it says.
type Row = (
    &'static str,
    &'static str,
    &'static [u8],
    &'static str,
    &'static str,
);

const HEADER_FAULTS: [Row; 1] = [(
    "binary_header_only",
    "A header and no value.",
    &[],
    "unexpected_end",
    "unexpected end",
)];

const BODY_FAULTS: [Row; 19] = [
    (
        "binary_cut_float",
        "A float with three of its eight bytes.",
        &[4, 0, 0, 0],
        "unexpected_end",
        "unexpected end",
    ),
    (
        "binary_cut_float32",
        "A 32-bit float with one of its four bytes.",
        &[12, 0],
        "unexpected_end",
        "unexpected end",
    ),
    (
        "binary_cut_int",
        "An int whose varint continues past the end.",
        &[3, 0x80],
        "bad_varint",
        "varint",
    ),
    (
        "binary_cut_bytes",
        "Bytes that say length 2 and have 1.",
        &[9, 2, 1],
        "bad_length",
        "past the end",
    ),
    (
        "binary_cut_string",
        "A new string of 2 bytes with 1 left.",
        &[5, 5, 65],
        "bad_length",
        "past the end",
    ),
    (
        "binary_list_count_past_end",
        "A list of 5 items with none left.",
        &[6, 5],
        "bad_length",
        "past the end",
    ),
    (
        "binary_trailing_bytes",
        "A whole value followed by one more byte.",
        &[0, 0],
        "trailing_bytes",
        "bytes after",
    ),
    (
        "binary_ints_trailing",
        "A packed int list shorter than the bytes after it.",
        &[10, 2, 2, 3, 0],
        "trailing_bytes",
        "bytes after",
    ),
    (
        "binary_string_not_utf8",
        "A string whose bytes are not UTF-8.",
        &[5, 5, 0xff, 0xfe],
        "bad_utf8",
        "UTF-8",
    ),
    (
        "binary_string_index_missing",
        "A string index (here 1) with nothing in the table yet.",
        &[5, 2],
        "bad_string_index",
        "isn't in the table",
    ),
    (
        "binary_string_index_after_end",
        "Index 1 when only one string is in the table.",
        &[6, 2, 5, 3, 97, 5, 2],
        "bad_string_index",
        "isn't in the table",
    ),
    (
        "binary_ref_bad_kind",
        "A reference kind must be lower case.",
        &[8, 3, b'X', 3, b'a'],
        "bad_reference",
        "isn't a kind",
    ),
    (
        "binary_ref_empty_id",
        "A reference id may not be empty.",
        &[8, 5, b'a', b'b', 1],
        "bad_reference",
        "isn't a reference's id",
    ),
    (
        "binary_ref_space_in_id",
        "A reference id may not hold whitespace.",
        &[8, 5, b'a', b'b', 3, b' '],
        "bad_reference",
        "isn't a reference's id",
    ),
    (
        "binary_ref_control_in_id",
        "A reference id may not hold control characters.",
        &[8, 5, b'a', b'b', 3, 0],
        "bad_reference",
        "isn't a reference's id",
    ),
    (
        "binary_stream_length_past_end",
        "A stream that says 9 bytes and has 1.",
        &[11, 9, 0, 0, 0, 1],
        "bad_length",
        "past the end",
    ),
    (
        "binary_stream_trailing",
        "A stream body longer than its items: the offset is within the stream's own body.",
        &[11, 2, 0, 0, 0, 0, 1],
        "trailing_bytes",
        "bytes after",
    ),
    (
        "binary_map_key_missing",
        "A map of one entry with no key.",
        &[7, 1],
        "bad_length",
        "past the end",
    ),
    (
        "binary_ints_count_past_end",
        "A packed int list of 2 items with one varint.",
        &[10, 2, 2],
        "bad_length",
        "past the end",
    ),
];

fn table(rows: &[Row]) -> Vec<Case> {
    rows.iter()
        .map(|&(name, why, body, kind, says)| fault(name, why, document(body), kind, says))
        .collect()
}

/// Every case of this module.
pub(crate) fn cases() -> Vec<Case> {
    let mut all = header();
    all.extend(table(&HEADER_FAULTS));
    all.extend(table(&BODY_FAULTS));
    all.extend(tags());
    all.extend(huge_and_varints());
    all.extend(depth());
    all.push(depth_ok());
    all
}

fn header() -> Vec<Case> {
    let bad_magic = |name: &str, why: &str, bytes: &[u8]| {
        fault(name, why, bytes.to_vec(), "bad_magic", "not Substrate Data")
    };
    let mut out = vec![
        bad_magic("binary_empty", "No bytes at all.", b""),
        bad_magic(
            "binary_short_magic",
            "Fewer than the four magic bytes.",
            b"SDA",
        ),
        bad_magic(
            "binary_wrong_magic",
            "Four bytes that are not `SDAT`.",
            b"XDAT\x01\x00",
        ),
        fault(
            "binary_no_version",
            "The magic and nothing after it.",
            b"SDAT".to_vec(),
            "unexpected_end",
            "unexpected end",
        ),
    ];
    for v in [0u8, 2, 255] {
        out.push(fault(
            &format!("binary_version_{v}"),
            "Only version 1 is read; any other version byte is an error at byte 5.",
            vec![b'S', b'D', b'A', b'T', v],
            "bad_version",
            "version",
        ));
    }
    out
}

fn tags() -> Vec<Case> {
    [15u8, 200, 255]
        .iter()
        .map(|&t| {
            fault(
                &format!("binary_unknown_tag_{t}"),
                "A tag byte that is none of 0..=14.",
                document(&[t]),
                "unknown_tag",
                "unknown tag",
            )
        })
        .collect()
}

fn huge_and_varints() -> Vec<Case> {
    let mut huge = vec![0xff; 8];
    huge.push(0x7f);
    let with_tag = |t: u8, rest: &[u8]| {
        let mut body = vec![t];
        body.extend_from_slice(rest);
        document(&body)
    };
    let mut out = Vec::new();
    for (name, what, t) in [
        (
            "list",
            "A list that says 2^63-1 items: an error, no allocation.",
            6,
        ),
        ("map", "A map that says 2^63-1 entries.", 7),
        ("ints", "A packed int list that says 2^63-1 items.", 10),
        ("floats", "A packed float list that says 2^63-1 items.", 13),
    ] {
        out.push(fault(
            &format!("binary_{name}_count_huge"),
            what,
            with_tag(t, &huge),
            "bad_length",
            "past the end",
        ));
    }
    let varint = |name: &str, why: &str, tail: Vec<u8>| {
        fault(name, why, with_tag(3, &tail), "bad_varint", "varint")
    };
    out.push(varint(
        "binary_varint_tenth_byte_two",
        "A varint's tenth byte may only be 0 or 1: 2 would not fit 64 bits.",
        cont(9, 2),
    ));
    out.push(varint(
        "binary_varint_eleven_bytes",
        "A varint longer than ten bytes.",
        cont(10, 1),
    ));
    out.push(varint(
        "binary_varint_never_ends",
        "A varint whose bytes all continue.",
        cont(12, 0x80),
    ));
    out
}

fn depth() -> Vec<Case> {
    let nested = |unit: &[u8]| {
        let mut body = Vec::new();
        for _ in 0..=MAX_DEPTH {
            body.extend_from_slice(unit);
        }
        body.push(0);
        document(&body)
    };
    vec![
        fault(
            "binary_depth_257_lists",
            "Lists nested 257 deep: more than MAX_DEPTH (256).",
            nested(&[6, 1]),
            "too_deep",
            "nested deeper",
        ),
        fault(
            "binary_depth_257_maps",
            "Maps nested 257 deep.",
            nested(&[7, 1, 3, b'k']),
            "too_deep",
            "nested deeper",
        ),
    ]
}

/// The one positive case at the depth limit.
fn depth_ok() -> Case {
    let mut body = Vec::new();
    for _ in 0..MAX_DEPTH {
        body.extend_from_slice(&[6, 1]);
    }
    body.push(0);
    let bytes = document(&body);
    decode(&bytes).expect("256 levels are fine");
    Case::new(
        "binary_depth_256",
        "Lists nested exactly 256 deep (MAX_DEPTH) decode; the innermost holds null.",
    )
    .binary(bytes)
}
