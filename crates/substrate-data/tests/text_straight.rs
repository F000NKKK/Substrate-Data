//! The straight text paths: `parse_ref` against `parse_value`, and the
//! streaming `Writer` against writing a value.

use substrate_data::text::{Writer, parse_ref, parse_value, to_string, to_string_pretty};
use substrate_data::{Bump, Map, Ref, Value};

const DOCS: &[&str] = &[
    "null",
    "true",
    "-0",
    "12345678901234567890",
    "-9223372036854775808",
    "1.5e3",
    r#""a\"b\u00e9\ud83d\ude00 plain""#,
    r#"{ a: 1, "b c": [1, 2.5, "x", null, {}], d: {}, e: [], }"#,
    "// c\n[1, /* x */ 2,\n   3]",
    "\u{feff}{\"k\": \"v\"}",
];

const BAD: &[&str] = &[
    "",
    "[1,",
    "{a: 1, a: 2}",
    "{\"a\": 1 \"b\"}",
    "01",
    "'x'",
    "\"line\nbreak\"",
    "\"\\ud800\"",
    "[1] 2",
];

#[test]
fn parse_ref_reads_what_parse_value_reads() {
    for text in DOCS {
        let arena = Bump::new();
        let borrowed = parse_ref(text, &arena).unwrap_or_else(|e| panic!("{text}: {e}"));
        assert_eq!(borrowed.to_value(), parse_value(text).unwrap(), "{text}");
    }
}

#[test]
fn parse_ref_refuses_what_parse_value_refuses_the_same_way() {
    for text in BAD {
        let arena = Bump::new();
        let a = parse_ref(text, &arena).unwrap_err();
        let b = parse_value(text).unwrap_err();
        assert_eq!((a.message, a.offset), (b.message, b.offset), "{text}");
    }
}

#[test]
fn a_big_map_refuses_a_repeated_key_and_shares_repeated_keys() {
    let keys: String = (0..40)
        .map(|i| ["k", &i.to_string(), ": 0,"].concat())
        .collect();
    let text = ["{", &keys].concat();
    let arena = Bump::new();
    assert!(parse_ref(&[&text, "k7: 0}"].concat(), &arena).is_err());
    assert!(parse_value(&[&text, "k39: 0}"].concat()).is_err());
    let list = parse_value(&["[", &text, "}, ", &text, "}]"].concat()).unwrap();
    assert_eq!(list.as_list().map(<[Value]>::len), Some(2));
}

fn sample() -> Value {
    let mut map = Map::new();
    map.insert("name", Value::from("a\"b"));
    map.insert("none", Value::List(vec![]));
    map.insert("empty", Value::Map(Map::new()));
    map.insert(
        "list",
        Value::List(vec![
            Value::Null,
            Value::Bool(true),
            Value::Int(-7),
            Value::Float(0.1),
            Value::Float(f64::NAN),
            Value::Ref(Ref::parse("file", "a/b").expect("a reference")),
            Value::Bytes(vec![1, 2, 3, 4]),
        ]),
    );
    Value::Map(map)
}

#[test]
fn the_writer_writes_a_value_as_to_string_does() {
    let v = sample();
    let mut compact = Writer::new();
    compact.value(&v);
    assert_eq!(compact.finish(), to_string(&v));
    let mut pretty = Writer::pretty();
    pretty.value(&v);
    assert_eq!(pretty.finish(), to_string_pretty(&v));
    assert!(to_string(&v).contains(r#""@file:a/b""#));
    assert!(to_string(&v).contains(r#""@bytes:AQIDBA==""#));
}

#[test]
fn the_writer_closes_containers_by_itself() {
    let mut w = Writer::pretty();
    w.map(2).key("a").list(2);
    w.int(1).list(0);
    w.key("b").str("x");
    assert_eq!(
        w.finish(),
        "{\n  \"a\": [\n    1,\n    []\n  ],\n  \"b\": \"x\"\n}\n"
    );
}
