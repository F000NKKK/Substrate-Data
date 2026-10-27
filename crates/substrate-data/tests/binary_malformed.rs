//! Malformed binary bytes: the header, every kind of cut and lie, and
//! nesting past the limit are errors that say what is wrong, never panics.

use substrate_data::binary::{
    DecodeError, Decoder, MAGIC, MAX_DEPTH, VERSION, decode, decode_ref, encode, tag,
};
use substrate_data::{Bump, Map, Ref, Value};

fn every_tag() -> Value {
    let mut m = Map::new();
    m.insert("null", Value::Null);
    m.insert(
        "flags",
        Value::List(vec![Value::Bool(true), Value::Bool(false)]),
    );
    m.insert("int", Value::Int(-77));
    m.insert("float", Value::Float(0.1));
    m.insert("text", Value::from("héllo"));
    m.insert("again", Value::from("héllo"));
    m.insert(
        "ref",
        Value::Ref(Ref::parse("file", "a/b").expect("a reference")),
    );
    m.insert("bytes", Value::Bytes(vec![0, 255]));
    m.insert(
        "nested",
        Value::List(vec![Value::List(vec![]), Value::Map(Map::new())]),
    );
    Value::Map(m)
}

#[test]
fn the_header_is_checked() {
    assert!(Decoder::open(b"nope").is_err());
    assert!(Decoder::open(b"SDA").is_err());
    let err = Decoder::open(b"SDAT\x09").unwrap_err();
    assert_eq!(
        err.to_string(),
        "byte 5: Substrate Data version 9, this reads version 1"
    );
    assert!(decode(b"SDAT").is_err());
    let arena = Bump::new();
    assert!(decode_ref(b"SDAT\x01", &arena).is_err());
    assert!(decode_ref(b"junk", &arena).is_err());
    let as_error: &dyn std::error::Error = &DecodeError {
        message: "m".into(),
        offset: 3,
    };
    assert_eq!(as_error.to_string(), "byte 3: m");
}

fn framed(body: &[u8]) -> Vec<u8> {
    let mut bytes = MAGIC.to_vec();
    bytes.push(VERSION);
    bytes.extend_from_slice(body);
    bytes
}

fn rejects(body: &[u8], want: &str) {
    let bytes = framed(body);
    let owned = decode(&bytes).expect_err("malformed");
    assert!(owned.message.contains(want), "{body:?}: {owned}");
    let arena = Bump::new();
    let borrowed = decode_ref(&bytes, &arena).expect_err("malformed");
    assert!(borrowed.message.contains(want), "{body:?}: {borrowed}");
}

#[test]
fn malformed_bytes_are_errors_that_say_what_is_wrong() {
    rejects(&[], "unexpected end");
    rejects(&[200], "unknown tag 200");
    rejects(&[tag::INT], "a varint cut short");
    rejects(&[tag::INT, 0x80], "a varint cut short");
    rejects(&[tag::FLOAT, 0, 0], "unexpected end");
    rejects(&[tag::FLOAT32, 0], "unexpected end");
    rejects(&[tag::LIST, 5], "length 5 past the end");
    rejects(&[tag::MAP, 5], "length 5 past the end");
    rejects(&[tag::FLOATS, 5], "length 5 past the end");
    rejects(&[tag::FLOATS32, 9, 0, 0], "length 9 past the end");
    rejects(&[tag::INTS, 4, 1], "length 4 past the end");
    rejects(&[tag::INTS, 2, 1, 0x80], "a varint cut short");
    rejects(&[tag::STR, 0], "string 0 isn't in the table");
    rejects(&[tag::STR, 0b101, b'a'], "bytes past the end");
    rejects(&[tag::STR, 0b11, 0xff], "a string isn't UTF-8");
    rejects(&[tag::BYTES, 5, 1], "bytes past the end");
    rejects(
        &[tag::REF, 0b1011, b'B', b'a', b'd', 0b11, b'x'],
        "isn't a kind",
    );
    rejects(
        &[tag::REF, 0b11, b'k', 0b11, b' '],
        "isn't a reference's id",
    );
    rejects(&[tag::REF, 0b11, b'k'], "a varint cut short");
    rejects(&[tag::MAP, 1, 0b11, b'k'], "unexpected end");
    rejects(&[tag::MAP, 1, 0, 0], "isn't in the table");
    rejects(&[tag::STREAM, 9, 0, 0, 0], "bytes past the end");
    rejects(&[tag::STREAM, 1, 0], "unexpected end");
    rejects(
        &[tag::STREAM, 3, 0, 0, 0, 1, tag::NULL, tag::NULL],
        "bytes after the value",
    );
    rejects(
        &[
            tag::INT,
            0xff,
            0xff,
            0xff,
            0xff,
            0xff,
            0xff,
            0xff,
            0xff,
            0xff,
            0x7f,
        ],
        "a varint",
    );
}

#[test]
fn nesting_is_limited_not_a_stack_overflow() {
    let mut body = Vec::new();
    for _ in 0..=MAX_DEPTH {
        body.extend_from_slice(&[tag::LIST, 1]);
    }
    body.push(tag::NULL);
    rejects(&body, "nested deeper than");
    let mut maps = Vec::new();
    for _ in 0..=MAX_DEPTH {
        maps.extend_from_slice(&[tag::MAP, 1, 0b11, b'k']);
    }
    maps.push(tag::NULL);
    rejects(&maps, "nested deeper than");
    let mut ok = Vec::new();
    for _ in 0..MAX_DEPTH {
        ok.extend_from_slice(&[tag::LIST, 1]);
    }
    ok.push(tag::NULL);
    assert!(decode(&framed(&ok)).is_ok());
}

#[test]
fn every_cut_of_a_document_is_an_error_never_a_panic() {
    let bytes = encode(&every_tag());
    let arena = Bump::new();
    for cut in 0..bytes.len() {
        assert!(decode(&bytes[..cut]).is_err(), "owned, cut at {cut}");
        assert!(
            decode_ref(&bytes[..cut], &arena).is_err(),
            "borrowed, cut at {cut}"
        );
    }
}
