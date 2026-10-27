//! Cases for bytes, references, lists, maps and the string table.

use substrate_data::{Map, Ref, Value};

use crate::case::Case;

/// Every case of this module.
pub(crate) fn cases() -> Vec<Case> {
    let mut all = bytes_and_refs();
    all.extend(lists());
    all.extend(maps());
    all.extend(string_table());
    all
}

fn reference(kind: &str, id: &str) -> Value {
    Value::Ref(Ref::parse(kind, id).expect("a reference"))
}

fn list(items: Vec<Value>) -> Value {
    Value::List(items)
}

fn map(entries: Vec<(&str, Value)>) -> Value {
    let mut m = Map::new();
    for (key, value) in entries {
        m.insert(key, value);
    }
    Value::Map(m)
}

fn bytes_and_refs() -> Vec<Case> {
    let all_bytes: Vec<u8> = (0..=255).collect();
    vec![
        Case::new("bytes_empty", "No bytes: tag 9, length 0. Text: `@bytes:`.")
            .all_forms(Value::Bytes(vec![])),
        Case::new(
            "bytes_small",
            "Four bytes: tag 9, a varint length, the raw bytes. Text is base64 with padding.",
        )
        .all_forms(Value::Bytes(vec![0, 1, 2, 255])),
        Case::new(
            "bytes_every_value",
            "Every byte value 0..=255: the length 256 is a two-byte varint.",
        )
        .all_forms(Value::Bytes(all_bytes)),
        Case::new(
            "ref_file",
            "A reference: tag 8, then its kind and id as table strings. Text: `@kind:id`.",
        )
        .all_forms(reference("file", "images/logo.png")),
        Case::new("ref_user", "A reference to a user.").all_forms(reference("user", "alice")),
        Case::new(
            "ref_colon_in_id",
            "An id may hold colons; only the first colon splits kind from id.",
        )
        .all_forms(reference("doc", "team:alice:notes")),
        Case::new(
            "ref_unicode_id",
            "An id is any text without whitespace or control characters.",
        )
        .all_forms(reference("doc", "guides/читать-меня")),
        Case::new("ref_kind_digits", "A kind is `[a-z][a-z0-9_]*`.")
            .all_forms(reference("project_2", "apps/notes")),
        Case::new("ref_at_in_id", "An id may start with `@`.")
            .all_forms(reference("user", "@alice")),
    ]
}

fn lists() -> Vec<Case> {
    vec![
        Case::new("list_empty", "An empty list: tag 6, count 0.").all_forms(list(vec![])),
        Case::new("list_mixed", "Values of every plain type in one list.").all_forms(list(vec![
            Value::Int(1),
            Value::from("a"),
            Value::Null,
            Value::Bool(true),
            Value::Float(0.5),
        ])),
        Case::new("list_nested", "Lists in lists.").all_forms(list(vec![
            list(vec![]),
            list(vec![Value::Int(1), list(vec![Value::Int(2)])]),
        ])),
        Case::new("list_of_ints", "A plain list of ints: a value built as a model value is written with tag 6 and a tag per item (a typed writer may pack it, see the packed cases).")
            .all_forms(list(vec![Value::Int(1), Value::Int(-2), Value::Int(300)])),
        Case::new("list_of_refs", "References in a list share the table: kind `file` is written once.")
            .all_forms(list(vec![
                reference("file", "a.png"),
                reference("file", "b.png"),
                reference("doc", "a.png"),
            ])),
    ]
}

fn maps() -> Vec<Case> {
    vec![
        Case::new("map_empty", "An empty map: tag 7, count 0.").all_forms(map(vec![])),
        Case::new(
            "map_file_order",
            "Keys keep the order they were written in; they are never sorted.",
        )
        .all_forms(map(vec![
            ("zeta", Value::Int(1)),
            ("alpha", Value::Int(2)),
            ("mid", Value::Int(3)),
        ])),
        Case::new(
            "map_nested",
            "A map of a user: strings, a list, a map, a ref and bytes.",
        )
        .all_forms(map(vec![
            ("name", Value::from("alice")),
            ("tags", list(vec![Value::from("admin"), Value::from("dev")])),
            (
                "profile",
                map(vec![("age", Value::Int(30)), ("active", Value::Bool(true))]),
            ),
            ("home", reference("project", "apps/notes")),
            ("avatar", Value::Bytes(vec![1, 2, 3])),
        ])),
        Case::new("map_unicode_key", "Keys are strings like any other.").all_forms(map(vec![
            ("имя", Value::from("алиса")),
            ("😀", Value::Int(1)),
        ])),
        Case::new("map_empty_key", "The empty string is a key.")
            .all_forms(map(vec![("", Value::Int(1))])),
        Case::new(
            "map_at_key",
            "A key starting with `@` is only a key, never a typed value.",
        )
        .all_forms(map(vec![("@x", Value::Int(1))])),
    ]
}

fn string_table() -> Vec<Case> {
    let many: Vec<Value> = (0..70)
        .map(|i| Value::from(format!("s{i}")))
        .chain([Value::from("s64"), Value::from("s0")])
        .collect();
    let user = |name: &str, id: i64| map(vec![("id", Value::Int(id)), ("name", Value::from(name))]);
    vec![
        Case::new("table_repeated_values", "A string repeated is written once; later uses are the table index shifted left (an even varint).")
            .all_forms(list(vec![Value::from("a"), Value::from("a"), Value::from("b"), Value::from("a")])),
        Case::new("table_repeated_keys", "Map keys are table strings too: `id` and `name` cost one varint after their first map.")
            .all_forms(list(vec![user("alice", 1), user("bob", 2), user("alice", 3)])),
        Case::new("table_key_is_value", "A key and a value that are the same string share one table entry.")
            .all_forms(map(vec![("name", Value::from("name"))])),
        Case::new("table_empty_string", "The empty string takes a table entry, and is referred to by it.")
            .all_forms(list(vec![Value::from(""), Value::from(""), map(vec![("", Value::from(""))])])),
        Case::new("table_kinds_and_ids", "A reference's kind and id are table strings: the id `a` of a doc reuses the file's `a`.")
            .all_forms(list(vec![
                reference("file", "a"),
                reference("file", "b"),
                reference("doc", "a"),
                Value::from("file"),
            ])),
        Case::new("table_two_byte_index", "Table index 64 needs a two-byte varint (64 << 1 = 128).")
            .all_forms(list(many)),
    ]
}
