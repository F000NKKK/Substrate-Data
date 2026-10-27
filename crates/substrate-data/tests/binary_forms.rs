//! The binary form: every tag written and read back as owned and borrowed
//! values, the step-by-step decoder, and malformed bytes as errors.

use substrate_data::binary::{
    DecodeError, Decoder, MAGIC, MAX_DEPTH, VERSION, Writer, decode, decode_ref, encode, tag,
};
use substrate_data::{Bump, Map, Ref, Schema, Stream, Value, read_bytes, to_bytes};

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
    m.insert("ref", Value::Ref(Ref::parse("file", "a/b").unwrap()));
    m.insert("bytes", Value::Bytes(vec![0, 255]));
    m.insert(
        "nested",
        Value::List(vec![Value::List(vec![]), Value::Map(Map::new())]),
    );
    Value::Map(m)
}

#[test]
fn a_value_round_trips_owned_and_borrowed() {
    let v = every_tag();
    let bytes = encode(&v);
    assert!(bytes.starts_with(MAGIC) && bytes[4] == VERSION);
    assert_eq!(decode(&bytes).unwrap(), v);
    let arena = Bump::new();
    assert_eq!(decode_ref(&bytes, &arena).unwrap().to_value(), v);
}

#[test]
fn the_writer_writes_packed_lists_and_floats_both_ways_read() {
    let mut w = Writer::new();
    w.map(7);
    w.key("ints").ints([1i64, -2, 300].into_iter());
    w.key("wide").floats(&[0.1, 0.2]);
    w.key("narrow").floats(&[0.5, 1.5]);
    w.key("f32s").floats32(&[0.25, 4.0]);
    w.key("one").float32(2.5);
    w.key("many").list(2).null().bool(true);
    w.key("bytes").bytes(&[9]);
    let bytes = w.finish();
    let owned = decode(&bytes).unwrap();
    let arena = Bump::new();
    let borrowed = decode_ref(&bytes, &arena).unwrap().to_value();
    assert_eq!(owned, borrowed);
    let ints = owned.get("ints").and_then(|i| i.as_list()).unwrap();
    assert_eq!(ints, [Value::Int(1), Value::Int(-2), Value::Int(300)]);
    assert_eq!(
        owned.get("wide").and_then(|i| i.as_list()).unwrap(),
        [Value::Float(0.1), Value::Float(0.2)]
    );
    assert_eq!(
        owned.get("narrow").and_then(|i| i.as_list()).unwrap(),
        [Value::Float(0.5), Value::Float(1.5)]
    );
    assert_eq!(
        owned.get("f32s").and_then(|i| i.as_list()).unwrap(),
        [Value::Float(0.25), Value::Float(4.0)]
    );
    assert_eq!(owned.get("one"), Some(&Value::Float(2.5)));
}

#[test]
fn a_stream_is_a_list_to_the_value_readers() {
    let s: Stream<u16> = (0..5u16).collect();
    let mut w = Writer::new();
    s.encode(&mut w);
    let bytes = w.finish();
    let want = Value::List((0..5).map(Value::Int).collect());
    assert_eq!(decode(&bytes).unwrap(), want);
    let arena = Bump::new();
    assert_eq!(decode_ref(&bytes, &arena).unwrap().to_value(), want);
    let mut broken = bytes.clone();
    broken.push(7);
    assert!(decode(&broken).is_err());
    let mut trailing = Writer::new();
    trailing.map(1).key("k").bool(true);
    assert!(decode(&trailing.finish()[..8]).is_err());
}

#[test]
fn the_step_by_step_decoder_answers_none_for_the_wrong_thing() {
    let mut w = Writer::new();
    w.list(11);
    w.null()
        .bool(true)
        .bool(false)
        .int(-5)
        .float(1.5)
        .float32(2.5)
        .int(7);
    w.str("s").bytes(&[1, 2]).ints([1i64].into_iter());
    w.floats(&[0.5]);
    let bytes = w.finish();
    let mut d = Decoder::open(&bytes).unwrap();
    assert_eq!(d.list(), Some(11));
    assert!(d.null());
    assert!(!d.null());
    assert_eq!(d.bool(), Some(true));
    assert_eq!(d.bool(), Some(false));
    assert_eq!(d.int(), Some(-5));
    assert_eq!(d.float(), Some(1.5));
    assert_eq!(d.float(), Some(2.5));
    assert_eq!(d.float(), Some(7.0));
    assert_eq!(d.int(), None);
    assert_eq!(d.str(), Some("s"));
    assert_eq!(d.str(), None);
    assert_eq!(d.bytes(), Some(&[1u8, 2][..]));
    assert_eq!(d.ints(), Some(1));
    assert_eq!(d.packed_int(), Some(1));
    assert_eq!(d.floats(), Some(vec![0.5]));
    d.leave();
    assert!(d.at_end());
}

#[test]
fn packed_float_lists_read_step_by_step_in_both_widths() {
    let mut w = Writer::new();
    w.floats(&[0.1, 0.2])
        .floats(&[0.5])
        .floats32(&[1.5, 2.5])
        .floats32(&[3.5]);
    w.float(f64::NAN).bool(true);
    let bytes = w.finish();
    let mut d = Decoder::open(&bytes).unwrap();
    assert_eq!(d.floats32(), None);
    assert_eq!(d.floats(), Some(vec![0.1, 0.2]));
    assert_eq!(d.floats32(), Some(vec![0.5]));
    assert_eq!(d.floats(), Some(vec![1.5, 2.5]));
    assert_eq!(d.floats32(), Some(vec![3.5]));
    assert_eq!(d.floats(), None);
    assert!(d.float().is_some_and(f64::is_nan));
    assert_eq!(d.float(), None);
    assert_eq!(d.skip(), Some(()));
    assert!(d.at_end());
    assert_eq!(d.skip(), None);
}

#[test]
fn maps_and_streams_step_by_step() {
    let mut w = Writer::new();
    w.map(2).key("a").int(1).key("b").str("x");
    let s: Stream<u8> = [1u8, 2].into_iter().collect();
    s.encode(&mut w);
    let bytes = w.finish();
    let mut d = Decoder::open(&bytes).unwrap();
    assert_eq!(d.list(), None);
    assert_eq!(d.map(), Some(2));
    assert_eq!(d.key(), Some("a"));
    assert_eq!(d.int(), Some(1));
    assert_eq!(d.key(), Some("b"));
    assert_eq!(d.skip(), Some(()));
    d.leave();
    assert!(d.stream().is_some());
    assert!(d.at_end());
    d.leave();
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
    let owned = decode(&bytes).unwrap_err();
    assert!(owned.message.contains(want), "{body:?}: {owned}");
    let arena = Bump::new();
    let borrowed = decode_ref(&bytes, &arena).unwrap_err();
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

#[test]
fn schemas_read_straight_or_fall_back_to_the_slow_way() {
    #[derive(Schema, Debug, PartialEq)]
    struct Record {
        name: String,
        tags: Vec<String>,
        score: f64,
        parts: Stream<u8>,
    }
    let r = Record {
        name: "n".into(),
        tags: vec!["a".into(), "a".into()],
        score: 0.3,
        parts: [1u8, 2].into_iter().collect(),
    };
    let bytes = to_bytes(&r);
    assert_eq!(read_bytes::<Record>(&bytes).unwrap(), r);
    // Written as plain values, read as the schema: the slow way.
    let slow = encode(&r.write());
    assert_eq!(read_bytes::<Record>(&slow).unwrap(), r);
    let wrong = encode(&Value::List(vec![]));
    assert!(read_bytes::<Record>(&wrong).is_err());
    assert!(read_bytes::<Record>(&[]).is_err());
    let err = read_bytes::<Record>(b"SDAT\x01\xc8").unwrap_err();
    assert_eq!(err[0].code, Some(substrate_data::Problem::BadBinary.into()));
}
