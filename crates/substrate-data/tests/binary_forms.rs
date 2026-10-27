//! The binary form: every tag written and read back as owned and borrowed
//! values, packed lists, streams and straight reads by schemas.

use substrate_data::binary::{MAGIC, VERSION, Writer, decode, decode_ref, encode};
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
