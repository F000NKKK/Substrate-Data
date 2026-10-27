//! Negative cases for frames and for the frame stream.

use std::io::{Cursor, ErrorKind};

use substrate_data::frame::read_shaped;
use substrate_data::io::read_frame_bytes;

use crate::case::{Case, Fault};
use crate::io_cases::stream;
use crate::schemas::shape_of;

/// Every case of this module.
pub(crate) fn cases() -> Vec<Case> {
    let mut all = frames();
    all.extend(streams());
    all
}

/// A negative case: `input` must read as `None` by the shape of `schema`.
fn frame_fault(name: &str, why: &str, schema: &'static str, input: &[u8]) -> Case {
    let shape = shape_of(schema).expect("a schema");
    assert_eq!(read_shaped(input, &shape), None, "{name}");
    Case::new(name, why).fault(Fault::Frame {
        schema,
        input: input.to_vec(),
    })
}

/// A fault: name, why, schema, the bytes.
type Row = (&'static str, &'static str, &'static str, &'static [u8]);

const FRAME_FAULTS: [Row; 14] = [
    (
        "frame_empty",
        "No bytes: not even the table length.",
        "uint",
        &[],
    ),
    (
        "frame_trailer_continues",
        "The backwards table length never ends (every byte continues).",
        "uint",
        &[0x80, 0x80],
    ),
    (
        "frame_trailer_too_long",
        "A backwards varint longer than ten bytes.",
        "uint",
        &[0x80; 11],
    ),
    (
        "frame_table_longer_than_frame",
        "The table length says 100 bytes in a frame of two.",
        "uint",
        &[0x00, 0x64],
    ),
    (
        "frame_body_left_over",
        "A number and a byte after it that nothing reads.",
        "uint",
        &[0x01, 0x00, 0x00],
    ),
    (
        "frame_missing_body",
        "The root number is not there (an empty body).",
        "uint",
        &[0x00],
    ),
    (
        "frame_bool_byte_two",
        "A bool is a byte 0 or 1.",
        "bool",
        &[0x02, 0x00],
    ),
    (
        "frame_string_index_missing",
        "A string index with no table entry.",
        "text",
        &[0x05, 0x00],
    ),
    (
        "frame_table_not_utf8",
        "A table string that is not UTF-8.",
        "text",
        &[0x00, 0x01, 0xff, 0x02],
    ),
    (
        "frame_table_string_cut",
        "A table entry that says 5 bytes and has 1.",
        "text",
        &[0x00, 0x05, 0x61, 0x02],
    ),
    (
        "frame_ints_width_zero",
        "A packed int list's width byte is 1..=64: 0 is none.",
        "uint_list",
        &[0x01, 0x00, 0x00],
    ),
    (
        "frame_ints_width_65",
        "Width 65 is too wide.",
        "uint_list",
        &[0x01, 0x41, 0x00, 0x00],
    ),
    (
        "frame_ints_count_lies",
        "200 numbers of 8 bits and no bytes for them.",
        "uint_list",
        &[0xc8, 0x01, 0x08, 0x00],
    ),
    (
        "frame_record_length_lies",
        "A record that says 127 bytes long in a frame without them.",
        "project_v1",
        &[0x7f, 0x00],
    ),
];

fn frames() -> Vec<Case> {
    FRAME_FAULTS
        .iter()
        .map(|&(name, why, schema, input)| frame_fault(name, why, schema, input))
        .collect()
}

/// A stream case: reading frames in order, then the error `kind`.
fn io_fault(name: &str, why: &str, input: Vec<u8>, kind: &'static str) -> Case {
    let mut r = Cursor::new(&input);
    let mut frames_before = 0;
    let err = loop {
        match read_frame_bytes(&mut r) {
            Ok(Some(_)) => frames_before += 1,
            Ok(None) => panic!("{name}: the stream ended cleanly"),
            Err(e) => break e,
        }
    };
    let got = match err.kind() {
        ErrorKind::UnexpectedEof => "unexpected_eof",
        ErrorKind::InvalidData => "invalid_data",
        other => panic!("{name}: {other:?}"),
    };
    assert_eq!(got, kind, "{name}");
    Case::new(name, why).fault(Fault::Io {
        input,
        kind,
        frames_before,
    })
}

fn streams() -> Vec<Case> {
    let good = stream(&[vec![0u8, 1, 2]]);
    let two_then_cut = [good.clone(), vec![0x01, 0x00]].concat();
    let max_plus_one = 0x1000_0001u32.to_le_bytes().to_vec();
    vec![
        io_fault(
            "io_cut_length",
            "The stream ends inside a length (two of its four bytes): an error, unlike an end between frames.",
            vec![0x01, 0x00],
            "unexpected_eof",
        ),
        io_fault(
            "io_cut_frame",
            "A length 5 and three bytes after it.",
            vec![5, 0, 0, 0, 1, 2, 3],
            "unexpected_eof",
        ),
        io_fault(
            "io_cut_second_length",
            "One good frame, then a length cut short: the first frame is read, then the error.",
            two_then_cut,
            "unexpected_eof",
        ),
        io_fault(
            "io_length_over_max",
            "A length above MAX_FRAME (268435456) is refused before any byte is read.",
            max_plus_one,
            "invalid_data",
        ),
        io_fault(
            "io_length_all_ones",
            "The biggest length, 0xffffffff.",
            vec![0xff; 4],
            "invalid_data",
        ),
    ]
}
