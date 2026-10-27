//! Text that only reads: the JSON5 conveniences, escapes and typed spellings.

use substrate_data::{Map, Value};

use crate::case::Case;

fn map(entries: Vec<(&str, Value)>) -> Value {
    let mut m = Map::new();
    for (key, value) in entries {
        m.insert(key, value);
    }
    Value::Map(m)
}

/// Every case of this module.
pub(crate) fn cases() -> Vec<Case> {
    let mut all = comments_and_layout();
    all.extend(strings());
    all
}

fn comments_and_layout() -> Vec<Case> {
    let release = map(vec![
        ("name", Value::from("Release 1.0")),
        ("parts", Value::List(vec![Value::Int(1), Value::Int(2)])),
    ]);
    vec![
        Case::new("syntax_comments", "Line and block comments are whitespace.")
            .text("// the task\n{ /* name */ \"name\": \"Release 1.0\", // why\n \"parts\": [1, 2] }")
            .value(release.clone()),
        Case::new("syntax_trailing_commas", "A trailing comma is allowed in a list and in a map.")
            .text("{\"name\": \"Release 1.0\", \"parts\": [1, 2,],}")
            .value(release.clone()),
        Case::new("syntax_unquoted_keys", "A key may be written without quotes.")
            .text("{ name: \"Release 1.0\", parts: [1, 2] }")
            .value(release),
        Case::new("syntax_bom", "A leading byte order mark is skipped.")
            .text("\u{feff}[1]")
            .value(Value::List(vec![Value::Int(1)])),
        Case::new("syntax_whitespace", "Spaces, tabs and line breaks between tokens are free.")
            .text(" \t\r\n[ 1 ,\n 2\t]\n")
            .value(Value::List(vec![Value::Int(1), Value::Int(2)])),
        Case::new("syntax_typed_strings_stay_strings", "The text parser keeps a typed spelling as the string it is; a schema (or the binary form) gives it its type.")
            .text("[\"@file:a.png\", \"@bytes:AQID\", \"@@text\"]")
            .value(Value::List(vec![
                Value::from("@file:a.png"),
                Value::from("@bytes:AQID"),
                Value::from("@@text"),
            ])),
    ]
}

fn strings() -> Vec<Case> {
    vec![
        Case::new(
            "syntax_escapes",
            "The JSON escapes, including `\\/` and `\\b`, `\\f`.",
        )
        .text(r#""\" \\ \/ \b \f \n \r \t A""#)
        .value(Value::from("\" \\ / \u{8} \u{c} \n \r \t A")),
        Case::new(
            "syntax_surrogate_pair",
            "A `\\u` surrogate pair is one character.",
        )
        .text(r#""🦀""#)
        .value(Value::from("🦀")),
        Case::new(
            "syntax_raw_unicode",
            "Non-ASCII characters may be written as they are.",
        )
        .text("\"héllo 日本語\"")
        .value(Value::from("héllo 日本語")),
    ]
}
