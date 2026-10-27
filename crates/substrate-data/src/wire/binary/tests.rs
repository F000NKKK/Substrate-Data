//! The binary form: round trips, and malformed bytes as errors.

use bumpalo::Bump;

use super::*;
use crate::model::borrowed::ValueRef;
use crate::model::value::{Map, Ref, Value};

fn sample() -> Value {
    let mut m = Map::new();
    m.insert("name", Value::Str("Ruby".into()));
    m.insert(
        "n",
        Value::List(vec![
            Value::Int(0),
            Value::Int(-1),
            Value::Int(i64::MAX),
            Value::Int(i64::MIN),
            Value::Float(0.5),
            Value::Null,
            Value::Bool(true),
            Value::Bool(false),
        ]),
    );
    m.insert(
        "logo",
        Value::Ref(Ref::parse("file", "images/logo.png").unwrap()),
    );
    m.insert("parent", Value::Ref(Ref::parse("user", "alice").unwrap()));
    m.insert("blob", Value::Bytes(vec![1, 2, 3]));
    m.insert("again", Value::Str("Ruby".into()));
    Value::Map(m)
}

/// Every string in `v` (keys too) onto `out`.
fn walk<'a>(v: &ValueRef<'a>, out: &mut Vec<&'a str>) {
    match v {
        ValueRef::Str(s) => out.push(s),
        ValueRef::List(l) => {
            for x in *l {
                walk(x, out);
            }
        }
        ValueRef::Map(m) => {
            for (k, x) in m.iter() {
                out.push(k);
                walk(x, out);
            }
        }
        _ => {}
    }
}

#[test]
fn bytes_round_trip_in_both_forms() {
    let v = Value::Bytes(vec![0, 1, 2, 255]);
    assert_eq!(decode(&encode(&v)).unwrap(), v);
    let text = crate::text::to_string(&v);
    assert_eq!(text, "\"@bytes:AAEC/w==\"");
    assert_eq!(
        crate::typed::classify("@bytes:AAEC/w==")
            .unwrap()
            .into_value(),
        v
    );
    assert!(
        decode(&[b'S', b'D', b'A', b'T', VERSION, tag::BYTES, 9]).is_err(),
        "a length past the end"
    );
}

#[test]
fn round_trips_every_type() {
    let v = sample();
    let bytes = encode(&v);
    assert_eq!(decode(&bytes).unwrap(), v);
    // "Ruby" is stored once.
    assert_eq!(bytes.windows(4).filter(|w| w == b"Ruby").count(), 1);
}

#[test]
fn the_borrowed_decode_reads_the_same_without_copying() {
    let v = sample();
    let bytes = encode(&v);
    let arena = Bump::new();
    let r = decode_ref(&bytes, &arena).unwrap();
    assert_eq!(r.to_value(), v);
    assert_eq!(r, v.borrow(&arena), "borrowing a Value gives the same");
    // Its strings point into the bytes.
    let span = bytes.as_ptr_range();
    let mut strings = Vec::new();
    walk(&r, &mut strings);
    assert!(!strings.is_empty() && strings.iter().all(|s| span.contains(&s.as_ptr())));
    // Malformed bytes are errors here too.
    for len in 0..bytes.len() {
        assert!(
            decode_ref(&bytes[..len], &arena).is_err(),
            "truncated at {len}"
        );
    }
}

#[test]
fn malformed_bytes_are_errors() {
    let bytes = encode(&sample());
    // Every truncation fails cleanly.
    for len in 0..bytes.len() {
        assert!(decode(&bytes[..len]).is_err(), "truncated at {len}");
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(
        decode(&extra)
            .unwrap_err()
            .message
            .contains("after the value")
    );
    assert!(
        decode(b"SDAT\x03\x00")
            .unwrap_err()
            .message
            .contains("version 3")
    );
    assert!(
        decode(b"SDAT\x01\x63")
            .unwrap_err()
            .message
            .contains("unknown tag")
    );
    assert!(
        decode(b"SDAT\x01\x05\x00")
            .unwrap_err()
            .message
            .contains("isn't in the table")
    );
    // A huge count with nothing behind it doesn't allocate.
    assert!(
        decode(b"SDAT\x01\x06\xff\xff\xff\xff\x0f")
            .unwrap_err()
            .message
            .contains("past the end")
    );
}

#[test]
fn deep_nesting_and_bad_references_are_errors() {
    let deep: Vec<u8> = b"SDAT\x01"
        .iter()
        .copied()
        .chain(std::iter::repeat_n([6u8, 1], 300).flatten())
        .chain([0])
        .collect();
    assert!(decode(&deep).unwrap_err().message.contains("nested deeper"));
    // A reference's kind and id are checked as they're read.
    assert!(
        decode(b"SDAT\x01\x08\x09File\x03x")
            .unwrap_err()
            .message
            .contains("isn't a kind")
    );
    assert!(
        decode(b"SDAT\x01\x08\x09file\x03 ")
            .unwrap_err()
            .message
            .contains("isn't a reference's id")
    );
    let arena = Bump::new();
    assert!(decode_ref(b"SDAT\x01\x08\x09File\x03x", &arena).is_err());
}

#[test]
fn a_kind_is_reused_from_its_string_and_a_bad_second_one_is_refused() {
    // Two references: the kind `file` written, then named by its index.
    let two = b"SDAT\x01\x06\x02\x08\x09file\x03x\x08\x00\x03y";
    let want = Value::List(vec![
        Value::Ref(Ref::parse("file", "x").unwrap()),
        Value::Ref(Ref::parse("file", "y").unwrap()),
    ]);
    assert_eq!(decode(two).unwrap(), want);
    let arena = Bump::new();
    assert!(decode_ref(two, &arena).is_ok());
    // The second reference's kind is a new, bad string.
    let bad = b"SDAT\x01\x06\x02\x08\x09file\x03x\x08\x07Bad\x03y";
    let err = decode(bad).unwrap_err();
    assert!(err.message.contains("isn't a kind"), "{err}");
    assert_eq!(err.offset, 16);
}

#[test]
fn a_shape_is_found_again_however_many_others_came_between() {
    let map = |keys: &[&str]| {
        let mut m = Map::new();
        for key in keys {
            m.insert(*key, Value::Int(1));
        }
        Value::Map(m)
    };
    let sets: [&[&str]; 9] = [
        &["alpha", "beta"],
        &["gamma"],
        &["delta"],
        &["epsilon"],
        &["zeta"],
        &["eta"],
        &["alpha", "beta"],
        &["beta", "alpha"],
        &["alpha", "beta"],
    ];
    let value = Value::List(sets.iter().map(|keys| map(keys)).collect());
    let bytes = encode(&value);
    let decoded = decode(&bytes).unwrap();
    assert_eq!(decoded, value);
    let Value::List(items) = decoded else {
        panic!("a list")
    };
    let order = |v: &Value| -> Vec<String> {
        let Value::Map(m) = v else { panic!("a map") };
        m.keys().map(str::to_owned).collect()
    };
    assert_eq!(order(&items[7]), ["beta", "alpha"]);
    // Each key is written once, in the first shape that has it.
    for key in ["alpha", "beta", "gamma"] {
        assert_eq!(
            bytes
                .windows(key.len())
                .filter(|w| *w == key.as_bytes())
                .count(),
            1
        );
    }
}

#[test]
fn a_repeated_key_is_refused_in_a_big_map_too() {
    let keys: Vec<String> = (0..40).map(|i| format!("k{i}")).collect();
    let mut w = Writer::new();
    w.map(keys.len() + 1);
    for key in keys.iter().chain([&keys[33]]) {
        w.key(key).int(1);
    }
    let bytes = w.finish();
    let err = decode(&bytes).unwrap_err();
    assert!(err.message.contains("k33"), "{err}");
    let arena = Bump::new();
    assert!(
        decode_ref(&bytes, &arena)
            .unwrap_err()
            .message
            .contains("k33")
    );
    // The same keys without the repeat read.
    let mut plain = Writer::new();
    plain.map(keys.len());
    for key in &keys {
        plain.key(key).int(1);
    }
    let Value::Map(map) = decode(&plain.finish()).unwrap() else {
        panic!("a map")
    };
    assert_eq!(map.len(), 40);
}
