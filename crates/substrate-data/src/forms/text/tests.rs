//! The parser and the writer on the syntax's corners.

use super::*;
use crate::model::value::{Path, Value};

#[test]
fn json5_subset() {
    let doc = parse("\u{feff}{\n  // comment\n  name: \"Ruby\", /* inline */ $schema: 1,\n  \"list\": [1, 2.5, -3e2, true, null,],\n}").unwrap();
    let m = doc.value.as_map().unwrap();
    assert_eq!(m.keys().collect::<Vec<_>>(), ["name", "$schema", "list"]);
    assert_eq!(
        doc.value.get("list").unwrap(),
        &Value::List(vec![
            Value::Int(1),
            Value::Float(2.5),
            Value::Float(-300.0),
            Value::Bool(true),
            Value::Null
        ])
    );
}

#[test]
fn locates_values_and_keys() {
    let text = "{\n  \"a\": [\n    1,\n    {\"b\": \"x\"}\n  ]\n}";
    let doc = parse(text).unwrap();
    let path = Path::root().key("a").index(1).key("b");
    assert_eq!(doc.locate(text, &path), LineCol { line: 4, col: 11 });
    assert_eq!(
        LineCol::of(text, doc.spans_at(&path).key.unwrap().start as usize),
        LineCol { line: 4, col: 6 }
    );
    // A path past the document: its deepest existing part.
    assert_eq!(
        doc.locate(text, &Path::root().key("a").index(9)),
        LineCol { line: 2, col: 8 }
    );
}

#[test]
fn errors_say_where() {
    let cases = [
        ("{\"a\": 1,, }", "1:9", "expected a key"),
        ("{\"a\": 1 \"b\": 2}", "1:9", "expected `,` or `}`"),
        ("{\"a\": 1, \"a\": 2}", "1:10", "already set"),
        ("[1, 2", "1:6", "found the end of the file"),
        ("\"ab\ncd\"", "1:4", "line break"),
        ("'x'", "1:1", "double quotes"),
        ("01", "1:1", "start with 0"),
        ("{} x", "1:4", "unexpected text"),
        ("/* never closed", "1:1", "unclosed"),
        ("\"\\q\"", "1:2", "unknown escape"),
        ("\"\\ud800\"", "1:2", "surrogate"),
        ("{\n  \"é\": tru }", "2:8", "expected a value"),
    ];
    for (text, at, msg) in cases {
        let e = parse(text).unwrap_err();
        assert_eq!(e.at.to_string(), at, "{text:?}: {e}");
        assert!(e.message.contains(msg), "{text:?}: {e}");
    }
    assert!(
        parse(&"[".repeat(MAX_DEPTH + 1))
            .unwrap_err()
            .message
            .contains("nested deeper")
    );
}

#[test]
fn strings_and_numbers() {
    let v = parse_value(
        r#"["a\"b\\c\n\u00e9\ud83d\ude00", 9223372036854775807, 9223372036854775808, 0.1]"#,
    )
    .unwrap();
    let l = v.as_list().unwrap();
    assert_eq!(l[0], Value::Str("a\"b\\c\né😀".into()));
    assert_eq!(l[1], Value::Int(i64::MAX));
    assert_eq!(l[2], Value::Float(9_223_372_036_854_775_808.0));
    assert_eq!(l[3], Value::Float(0.1));
}

#[test]
fn writes_what_it_reads() {
    let text = "{\n  \"name\": \"Ruby \\\"red\\\"\\n\",\n  \"n\": [1, 2.0, 0.1, -5],\n  \"empty\": {},\n  \"none\": [],\n  \"tab\\u0001\": null\n}\n";
    let v = parse_value(text).unwrap();
    assert_eq!(parse_value(&to_string(&v)).unwrap(), v);
    let pretty = to_string_pretty(&v);
    assert_eq!(parse_value(&pretty).unwrap(), v);
    assert!(
        pretty.starts_with("{\n  \"name\": \"Ruby \\\"red\\\"\\n\",\n  \"n\": [\n    1,\n    2.0,"),
        "{pretty}"
    );
    assert_eq!(
        to_string(&v),
        r#"{"name":"Ruby \"red\"\n","n":[1,2.0,0.1,-5],"empty":{},"none":[],"tab\u0001":null}"#
    );
}

#[test]
fn floats_read_back_as_the_same_floats() {
    for f in [1e16, -2e18, 1e300, 1e-7, 0.1, 1.0, -0.0, f64::MAX] {
        let text = to_string(&Value::Float(f));
        assert_eq!(
            parse_value(&text).unwrap(),
            Value::Float(f),
            "{f} as {text}"
        );
    }
}

#[test]
fn typed_values_are_strings_in_text() {
    let v = Value::List(vec![
        Value::Ref(crate::Ref::parse("file", "images/logo.png").unwrap()),
        Value::Bytes(vec![1, 2, 3]),
        Value::Str("@@not a ref".into()),
    ]);
    assert_eq!(
        to_string(&v),
        r#"["@file:images/logo.png","@bytes:AQID","@@not a ref"]"#
    );
}
