//! Comments kept as data: read in, written back.

use super::with_comments;
use crate::model::value::Value;

fn s(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::to_string)
}

#[test]
fn comments_become_notes() {
    let text = "// The intro doc.\n// Short.\n{\n  // How long.\n  pages: 3,\n  /* Its cover. */\n  cover: { file: \"@file:cover.png\" },\n  list: [\n    // First.\n    { a: 1 },\n    // A number.\n    2,\n  ],\n  // Left at the end.\n}";
    let v = with_comments(text).unwrap();
    assert_eq!(
        s(&v, "comment").as_deref(),
        Some("The intro doc.\nShort.\nA number.\nLeft at the end.")
    );
    assert_eq!(s(&v, "$pages").as_deref(), Some("How long."));
    assert_eq!(
        s(v.get("cover").unwrap(), "comment").as_deref(),
        Some("Its cover.")
    );
    let first = &v.get("list").unwrap().as_list().unwrap()[0];
    assert_eq!(s(first, "comment").as_deref(), Some("First."));
    let keys: Vec<&str> = v.as_map().unwrap().keys().collect();
    assert_eq!(keys, ["comment", "$pages", "pages", "cover", "list"]);
}

#[test]
fn text_without_comments_is_its_value() {
    let text = "{ a: \"// not a comment\", b: [1, 2] }";
    assert_eq!(
        with_comments(text).unwrap(),
        super::super::parse_value(text).unwrap()
    );
}
