//! Patching keeps what didn't change: comments, order, spelling, layout.

use super::format::number;
use super::*;
use crate::model::value::{Map, Value};
use crate::text::parse_value;

fn edit(text: &str, change: impl FnOnce(&mut Map)) -> String {
    let Value::Map(mut v) = parse_value(text).unwrap() else {
        panic!("a map")
    };
    change(&mut v);
    let next = Value::Map(v);
    let out = patch(text, &next);
    assert!(same(&parse_value(&out).unwrap(), &next), "{out}");
    out
}

const FILE: &str = r#"// A document.
{
  "id": "intro", // its id
  /* What it is. */
  "page": {
    "margin": 4.0,
    "count": 15
  },
  "states": {
    "properties": {
      // On or off.
      "lit": ["false", "true"],
    },
  },
}
"#;

fn map_at<'a>(m: &'a mut Map, key: &str) -> &'a mut Map {
    match m.get_mut(key) {
        Some(Value::Map(inner)) => inner,
        _ => panic!("no map `{key}`"),
    }
}

#[test]
fn what_didnt_change_keeps_its_text() {
    assert_eq!(edit(FILE, |_| {}), FILE);
    let out = edit(FILE, |m| {
        map_at(m, "page").insert("count", Value::Int(7));
    });
    assert_eq!(out, FILE.replace("\"count\": 15", "\"count\": 7"));
}

#[test]
fn a_new_key_goes_after_the_others() {
    let out = edit(FILE, |m| {
        map_at(m, "page").insert("size", Value::Str("a4".into()));
    });
    assert!(
        out.contains("\"count\": 15,\n    \"size\": \"a4\"\n  },"),
        "{out}"
    );
    assert!(out.contains("// its id"));
}

#[test]
fn a_removed_key_takes_its_comments() {
    let out = edit(FILE, |m| {
        m.remove("page");
    });
    assert!(!out.contains("What it is."), "{out}");
    assert!(
        out.contains("\"id\": \"intro\", // its id") && out.contains("// On or off."),
        "{out}"
    );
    assert_eq!(
        edit("{\n  \"a\": 1, // one\n  \"b\": 2\n}", |m| {
            m.remove("b");
        }),
        "{\n  \"a\": 1 // one\n}"
    );
    assert_eq!(
        edit("{\n  \"a\": 1 // one\n}", |m| {
            m.insert("b", Value::Int(2));
        }),
        "{\n  \"a\": 1, // one\n  \"b\": 2\n}"
    );
}

#[test]
fn a_renamed_key_keeps_its_place_and_comments() {
    let out = edit(FILE, |m| {
        let props = map_at(map_at(m, "states"), "properties");
        let lit = props.remove("lit").unwrap_or(Value::Null);
        props.insert("on", lit);
    });
    assert!(
        out.contains("// On or off.\n      \"on\": [\"false\", \"true\"],"),
        "{out}"
    );
    let text = "{\n  a: 1,\n  b: 2,\n  c: 3\n}";
    let mut next = Map::new();
    for (k, v) in [("a", 1), ("x", 2), ("c", 3)] {
        next.insert(k, Value::Int(v));
    }
    assert_eq!(
        patch(text, &Value::Map(next)),
        "{\n  a: 1,\n  \"x\": 2,\n  c: 3\n}"
    );
}

#[test]
fn list_items_added_and_removed_keep_the_others() {
    let text = "{\n  \"list\": [\n    // first\n    { \"a\": 1 },\n    { \"b\": 2 }\n  ]\n}";
    let list = |m: &mut Map| match m.get_mut("list") {
        Some(Value::List(l)) => std::mem::take(l),
        _ => Vec::new(),
    };
    let out = edit(text, |m| {
        let mut l = list(m);
        l.push(parse_value("{ c: 3 }").unwrap());
        m.insert("list", Value::List(l));
    });
    assert!(out.contains("// first"), "{out}");
    let removed = edit(text, |m| {
        let mut l = list(m);
        l.remove(0);
        m.insert("list", Value::List(l));
    });
    assert!(
        !removed.contains("first") && removed.contains("{ \"b\": 2 }"),
        "{removed}"
    );
}

#[test]
fn number_keys_keep_their_order() {
    let text = "{\n  \"levels\": {\n    \"0\": [0, 0, 0], // rest\n    \"0.5\": [45, 0, 0],\n    \"1\": [0, 0, 0]\n  }\n}";
    let out = edit(text, |m| {
        map_at(m, "levels").insert("0.5", parse_value("[90, 0, 0]").unwrap());
    });
    assert_eq!(out, text.replace("[45, 0, 0]", "[90, 0, 0]"));
}

#[test]
fn json_keeps_order_and_source_strings() {
    let v = parse_value(r#"{ "1": "@file:images/logo.png", "0.5": [1.5, null, true] }"#).unwrap();
    assert_eq!(
        to_json(&v),
        r#"{"1":"@file:images/logo.png","0.5":[1.5,null,true]}"#
    );
}

#[test]
fn short_values_go_on_one_line() {
    let v = parse_value(
        r#"{ "margins": [0, 0, 16, 16.5], "sections": { "intro": { "title": "x" } } }"#,
    )
    .unwrap();
    assert_eq!(
        format(&v, 0),
        "{\n  \"margins\": [0, 0, 16, 16.5],\n  \"sections\": {\n    \"intro\": { \"title\": \"x\" }\n  }\n}"
    );
    assert_eq!(patch("{ oops", &Value::Int(1)), "1\n");
}

#[test]
fn floats_are_written_exactly() {
    for f in [
        0.1,
        1.0 / 3.0,
        -0.000_123_456_789,
        f64::from(0.1f32),
        1e-7,
        1e20,
        123_456.789_012_345,
        f64::MIN_POSITIVE,
        f64::MAX,
    ] {
        let s = number(f);
        assert_eq!(parse_value(&s).unwrap(), Value::Float(f), "{f} as {s}");
    }
    // A whole float stays a float, and `-0.0` keeps its sign.
    assert_eq!(number(3.0), "3.0");
    assert_eq!(number(-42.0), "-42.0");
    assert_eq!(number(-0.0), "-0.0");
    assert_eq!(number(0.5), "0.5");
    let Value::Float(z) = parse_value(&number(-0.0)).unwrap() else {
        panic!("a float")
    };
    assert!(z.is_sign_negative());
    let out = edit("{\n  \"at\": [0, 0, 0],\n  \"n\": 1\n}", |m| {
        m.insert(
            "at",
            Value::List(vec![Value::Float(0.123_456_7), Value::Float(2.0)]),
        );
    });
    assert!(out.contains("[0.1234567, 2.0]"), "{out}");
    // A zero turned negative is written so; an integer the source
    // spells stays as it is when the number didn't change.
    let signed = edit("{\n  \"z\": 0,\n  \"n\": 1\n}", |m| {
        m.insert("z", Value::Float(-0.0));
        m.insert("n", Value::Float(1.0));
    });
    assert_eq!(signed, "{\n  \"z\": -0.0,\n  \"n\": 1\n}");
}

#[test]
fn format_picks_one_line_or_many_by_what_fits() {
    let v = parse_value(
        r#"{
            "empty_list": [], "empty_map": {}, "mixed": [1, "a"], "nums": [1, true, 2.5],
            "ref": "@file:a.png", "long": [100000000000, 200000000000, 300000000000, 400000000000, 500000000000, 600000000000],
            "pair": { "a": [1, 2], "b": { "c": 1 } },
            "flat": { "a": [1, 2], "b": "x\"y", "c": null }
        }"#,
    )
    .unwrap();
    let out = format(&v, 1);
    let lines: Vec<&str> = out.lines().collect();
    assert!(lines.contains(&"    \"empty_list\": [],"));
    assert!(lines.contains(&"    \"empty_map\": {},"));
    assert!(lines.contains(&"    \"nums\": [1, true, 2.5],"));
    assert!(lines.contains(&"    \"flat\": { \"a\": [1, 2], \"b\": \"x\\\"y\", \"c\": null }"));
    // A list with a string in it, a map with a map in it, and a line past 72 columns: an entry a line.
    assert!(out.contains("\"mixed\": [\n      1,\n      \"a\"\n    ]"));
    assert!(out.contains("\"b\": { \"c\": 1 }"));
    assert!(out.contains("\"pair\": {\n"));
    assert!(out.contains("\"long\": [\n      100000000000,"));
    assert_eq!(parse_value(&out).unwrap(), v);
}
