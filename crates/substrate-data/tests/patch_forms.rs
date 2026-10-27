//! Patching and formatting a document: what didn't change keeps its text,
//! values are laid out and compared in one spelling.

use substrate_data::patch::{format, patch, same, to_json};
use substrate_data::text::parse_value;
use substrate_data::{Map, Ref, Value};

fn value(text: &str) -> Value {
    parse_value(text).expect("a document")
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

fn members_of(m: &mut Map) -> &mut Vec<Value> {
    match m.get_mut("members") {
        Some(Value::List(l)) => Some(l),
        _ => None,
    }
    .expect("members")
}

fn nested_of(m: &mut Map) -> &mut Map {
    match m.get_mut("nested") {
        Some(Value::Map(n)) => Some(n),
        _ => None,
    }
    .expect("nested")
}

/// `FILE` patched after `change` to its value, which the patch must read back as.
fn edit(change: impl FnOnce(&mut Map)) -> String {
    let mut map = value(FILE).as_map().expect("a map").clone();
    change(&mut map);
    let next = Value::Map(map);
    let patched = patch(FILE, &next);
    assert!(same(&value(&patched), &next), "{patched}");
    patched
}

#[test]
fn an_unchanged_document_is_returned_as_it_was() {
    assert_eq!(edit(|_| {}), FILE);
    assert_eq!(patch("{ }", &value("{ }")), "{ }");
}

#[test]
fn list_items_are_added_and_removed_in_place() {
    assert_eq!(
        edit(|m| members_of(m).push(Value::from("cy"))),
        FILE.replace("\"bob\",\n", "\"bob\",\n    \"cy\",\n")
    );
    assert_eq!(
        edit(|m| {
            members_of(m).remove(0);
        }),
        FILE.replace("    \"ann\", // first\n", "")
    );
    assert_eq!(
        edit(|m| members_of(m)[1] = Value::from("dee")),
        FILE.replace("bob", "dee")
    );
    assert_eq!(
        edit(|m| members_of(m).clear()),
        FILE.replace("    \"ann\", // first\n    \"bob\",\n", "")
    );
}

#[test]
fn keys_are_added_removed_and_replaced() {
    assert_eq!(
        edit(|m| {
            m.remove("name");
        }),
        FILE.replace("  \"name\": \"docs\", // its name\n", "")
    );
    assert_eq!(
        edit(|m| {
            m.insert("name", Value::Int(3));
            m.insert("extra", Value::List(vec![Value::Null]));
        }),
        FILE.replace("\"docs\"", "3")
            .replace("},\n}\n", "},\n  \"extra\": [\n    null\n  ],\n}\n")
    );
    assert_eq!(
        edit(|m| {
            m.insert("nested", Value::from("flat"));
        }),
        FILE.replace("{ \"a\": [1, 2], \"b\": {} }", "\"flat\"")
    );
}

#[test]
fn nested_objects_are_edited_entry_by_entry() {
    assert_eq!(
        edit(|m| {
            let n = nested_of(m);
            n.insert("c", Value::Bool(true));
            n.remove("a");
        }),
        FILE.replace(
            "{ \"a\": [1, 2], \"b\": {} }",
            "{\n    \"b\": {},\n    \"c\": true\n  }"
        )
    );
    assert_eq!(
        edit(|m| nested_of(m).retain(|_, _| false)),
        FILE.replace("{ \"a\": [1, 2], \"b\": {} }", "{}")
    );
}

#[test]
fn a_document_of_another_shape_is_written_fresh() {
    assert_eq!(patch(FILE, &value("[1, 2]")), "// A project.\n[1, 2]\n");
    assert_eq!(patch("/* c */ 5 // x", &Value::Int(6)), "/* c */ 6 // x");
    assert_eq!(patch("[1, /* two */ 2]", &value("[1, 3]")), "[1, 3]");
    assert_eq!(patch("not json", &Value::Int(1)), "1\n");
}
