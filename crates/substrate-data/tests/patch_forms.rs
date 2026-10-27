//! Patching and formatting a document: what didn't change keeps its text,
//! values are laid out and compared in one spelling.

use substrate_data::patch::{format, patch, same, to_json};
use substrate_data::text::parse_value;
use substrate_data::{Map, Ref, Value};

fn value(text: &str) -> Value {
    parse_value(text).unwrap()
}

#[test]
fn formatting_lays_values_out_small_ones_inline() {
    assert_eq!(format(&value("[]"), 0), "[]");
    assert_eq!(format(&value("{}"), 0), "{}");
    assert_eq!(format(&value("[1, 2.5, true]"), 0), "[1, 2.5, true]");
    assert_eq!(
        format(&value(r#"{a: 1, b: "x"}"#), 0),
        r#"{ "a": 1, "b": "x" }"#
    );
    assert_eq!(format(&Value::Null, 0), "null");
    assert_eq!(format(&Value::Float(f64::NAN), 0), "null");
    assert_eq!(format(&Value::Float(-0.0), 0), "-0.0");
    let nested = format(&value(r#"{a: {b: [1, {c: 2}], d: ["x"]}}"#), 0);
    assert_eq!(
        nested,
        "{\n  \"a\": {\n    \"b\": [\n      1,\n      { \"c\": 2 }\n    ],\n    \"d\": [\n      \"x\"\n    ]\n  }\n}"
    );
    assert_eq!(
        parse_value(&nested).unwrap(),
        value(r#"{a: {b: [1, {c: 2}], d: ["x"]}}"#)
    );
    let long = Value::List((0..40).map(Value::Int).collect());
    assert!(format(&long, 0).contains('\n'));
    let typed = Value::List(vec![
        Value::Ref(Ref::parse("file", "x").unwrap()),
        Value::Bytes(vec![1]),
    ]);
    assert!(format(&typed, 1).contains("@file:x"));
}

#[test]
fn compact_json_keeps_order_and_spelling() {
    let v = value(r#"{ b: [1, 2.5, "x\n"], a: { c: null }, f: 1e400 }"#);
    assert_eq!(
        to_json(&v),
        r#"{"b":[1,2.5,"x\n"],"a":{"c":null},"f":null}"#
    );
    assert_eq!(to_json(&Value::Float(f64::NAN)), "null");
    assert_eq!(to_json(&Value::Bool(true)), "true");
}

#[test]
fn values_are_the_same_however_a_number_is_spelled() {
    assert!(same(&Value::Int(1), &Value::Float(1.0)));
    assert!(!same(&Value::Float(-0.0), &Value::Int(0)));
    assert!(!same(&Value::Int(1), &Value::Str("1".into())));
    assert!(!same(&Value::Null, &Value::Bool(false)));
    assert!(same(&value("[1, {a: 2}]"), &value("[1.0, {a: 2}]")));
    assert!(!same(&value("[1, 2]"), &value("[1]")));
    assert!(!same(&value("[1, 2]"), &value("[1, 3]")));
    assert!(!same(&value("{a: 1}"), &value("{b: 1}")));
    assert!(!same(&value("{a: 1}"), &value("{a: 1, b: 2}")));
    assert!(same(&Value::Null, &Value::Null));
    assert!(same(&Value::Str("a".into()), &Value::Str("a".into())));
}

const FILE: &str = "// A project.\n{\n  \"name\": \"docs\", // its name\n  /* Members. */\n  \"members\": [\n    \"ann\", // first\n    \"bob\",\n  ],\n  \"nested\": { \"a\": [1, 2], \"b\": {} },\n}\n";

fn edit(change: impl FnOnce(&mut Map)) -> String {
    let Value::Map(mut v) = value(FILE) else {
        panic!("a map")
    };
    change(&mut v);
    let next = Value::Map(v);
    let out = patch(FILE, &next);
    assert!(same(&parse_value(&out).unwrap(), &next), "{out}");
    out
}

#[test]
fn an_unchanged_document_is_returned_as_it_was() {
    assert_eq!(edit(|_| {}), FILE);
    assert_eq!(patch("{ }", &value("{ }")), "{ }");
}

#[test]
fn list_items_are_added_and_removed_in_place() {
    let out = edit(|m| {
        let Some(Value::List(l)) = m.get_mut("members") else {
            panic!()
        };
        l.push(Value::from("cy"));
    });
    assert!(out.contains("// first") && out.contains("\"cy\""), "{out}");
    let out = edit(|m| {
        let Some(Value::List(l)) = m.get_mut("members") else {
            panic!()
        };
        l.remove(0);
    });
    assert!(out.contains("\"bob\"") && !out.contains("\"ann\""), "{out}");
    let out = edit(|m| {
        let Some(Value::List(l)) = m.get_mut("members") else {
            panic!()
        };
        l[1] = Value::from("dee");
    });
    assert!(out.contains("// first") && out.contains("dee"), "{out}");
    let out = edit(|m| {
        let Some(Value::List(l)) = m.get_mut("members") else {
            panic!()
        };
        l.clear();
    });
    assert!(
        !out.contains("ann") && out.contains("/* Members. */"),
        "{out}"
    );
}

#[test]
fn keys_are_added_removed_and_replaced() {
    let out = edit(|m| {
        m.remove("name");
    });
    assert!(!out.contains("docs"), "{out}");
    let out = edit(|m| {
        m.insert("name", Value::Int(3));
        m.insert("extra", Value::List(vec![Value::Null]));
    });
    assert!(
        out.contains("// its name") || out.contains("\"extra\""),
        "{out}"
    );
    let out = edit(|m| {
        m.insert("nested", Value::from("flat"));
    });
    assert!(out.contains("\"flat\""), "{out}");
    let out = edit(|m| {
        let Some(Value::Map(n)) = m.get_mut("nested") else {
            panic!()
        };
        n.insert("b", Value::Map(Map::new()));
        n.insert("c", Value::Bool(true));
        n.remove("a");
    });
    assert!(out.contains("\"c\""), "{out}");
    let out = edit(|m| {
        let Some(Value::Map(n)) = m.get_mut("nested") else {
            panic!()
        };
        let all: Vec<_> = n.keys().map(str::to_owned).collect();
        for k in all {
            n.remove(&k);
        }
    });
    assert!(out.contains("{}"), "{out}");
}

#[test]
fn a_document_of_another_shape_is_written_fresh() {
    let next = value("[1, 2]");
    assert_eq!(parse_value(&patch(FILE, &next)).unwrap(), next);
    let out = patch("/* c */ 5 // x", &Value::Int(6));
    assert!(out.contains('6'), "{out}");
    let out = patch("[1, /* two */ 2]", &value("[1, 3]"));
    assert!(out.contains("/* two */") || out.contains('3'), "{out}");
    assert_eq!(
        parse_value(&patch("not json", &Value::Int(1))).unwrap(),
        Value::Int(1)
    );
}
