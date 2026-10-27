//! Cases for what version 2 adds to the binary form (small ints, string
//! numbers in a tag, shapes, records, packed lists of a model value, a
//! float that is an `f32`), and for reading version 1 and writing version 2.

use substrate_data::{Map, Ref, Value, binary};

use crate::case::Case;
use crate::v1::V1Writer;

/// Every case of this module.
pub(crate) fn cases() -> Vec<Case> {
    let mut all = ints_and_strings();
    all.extend(lists());
    all.extend(shapes());
    all.extend(upgrades());
    all
}

fn map(entries: Vec<(&str, Value)>) -> Value {
    let mut m = Map::new();
    for (key, value) in entries {
        m.insert(key, value);
    }
    Value::Map(m)
}

fn ints(items: &[i64]) -> Value {
    Value::List(items.iter().map(|&n| Value::Int(n)).collect())
}

fn floats(items: &[f64]) -> Value {
    Value::List(items.iter().map(|&f| Value::Float(f)).collect())
}

fn text(s: &str) -> Value {
    Value::from(s)
}

fn ints_and_strings() -> Vec<Case> {
    let edges = Value::List(vec![
        Value::Int(-17),
        Value::Int(-16),
        Value::Int(0),
        Value::Int(111),
        Value::Int(112),
        Value::Null,
    ]);
    let many: Vec<Value> = (0..40).map(|i| text(&format!("s{i}"))).collect();
    let mut again = many.clone();
    again.extend([
        many[0].clone(),
        many[31].clone(),
        many[32].clone(),
        many[39].clone(),
    ]);
    vec![
        Case::new(
            "int_inline_edges",
            "Version 2: an int from -16 to 111 is one byte, the tag 128 + (n + 16); -17 and 112 take tag 3 and a varint. The list has a null in it so that it is not packed.",
        )
        .all_forms(edges),
        Case::new(
            "string_index_in_tag",
            "Version 2: a string already in the table whose index is under 32 is one byte, the tag 32 + index; from 32 on it is tag 5 and the index as a varint. Here the 40 strings are written, then strings 0, 31, 32 and 39 again.",
        )
        .all_forms(Value::List(again)),
    ]
}

fn lists() -> Vec<Case> {
    vec![
        Case::new(
            "list_ints_packed_by_value_writer",
            "Version 2: a list of ints, all of them ints, is tag 10 (count, then a varint each) in a model value too; version 1 wrote a tag per item.",
        )
        .all_forms(ints(&[1, -2, 300, i64::MAX, i64::MIN])),
        Case::new(
            "list_ints_single",
            "A list of one int is packed as well: tag 10, count 1.",
        )
        .all_forms(ints(&[5])),
        Case::new(
            "list_floats_narrow",
            "Version 2: a list of floats, all of them floats, is tag 14 when every one is an f32 exactly (count, 4 bytes each) ...",
        )
        .all_forms(floats(&[0.5, -1.5, 0.0, -0.0, 3.0e38])),
        Case::new(
            "list_floats_wide",
            "... and tag 13 (8 bytes each) when one is not: 0.1 is not an f32.",
        )
        .all_forms(floats(&[0.5, 0.1])),
        Case::new(
            "list_floats_infinite",
            "Infinities are f32 exactly, so they narrow: tag 14. Binary only: text writes them as null.",
        )
        .value(floats(&[f64::INFINITY, f64::NEG_INFINITY, 0.5]))
        .binary_v2(binary::encode(&floats(&[f64::INFINITY, f64::NEG_INFINITY, 0.5]))),
        Case::new(
            "list_floats_with_nan",
            "A NaN is never narrowed (it makes the whole list wide), so its bits are kept as they are. Binary only: text writes a NaN as null.",
        )
        .value(floats(&[f64::NAN, 0.5]))
        .binary_v2(binary::encode(&floats(&[f64::NAN, 0.5]))),
        Case::new(
            "float_single_narrow",
            "Version 2: a float that is an f32 exactly (here 0.1f32 widened to f64) is tag 12 and four bytes; 0.1 as an f64 is not, and stays tag 4.",
        )
        .all_forms(Value::List(vec![
            Value::Float(f64::from(0.1f32)),
            Value::Float(0.1),
            Value::Null,
        ])),
        Case::new(
            "list_mixed_numbers",
            "A list of ints and floats is not packed: a tag per item, an int inline.",
        )
        .all_forms(Value::List(vec![Value::Int(1), Value::Float(0.5)])),
    ]
}

fn user(name: &str, age: i64) -> Value {
    map(vec![("name", text(name)), ("age", Value::Int(age))])
}

fn shapes() -> Vec<Case> {
    let mut all = maps();
    all.extend(records());
    all
}

fn maps() -> Vec<Case> {
    vec![
        Case::new(
            "map_shaped",
            "Version 2: a map is tag 15, then its keys once as a shape (marker `count << 1 | 1`, then each key as a string), then one value per key.",
        )
        .all_forms(user("alice", 30)),
        Case::new(
            "map_shape_reused",
            "A second map with the same keys, in the same order, names the shape by its number (marker `number << 1`) and writes no keys.",
        )
        .all_forms(map(vec![("first", user("alice", 30)), ("second", user("bob", 25))])),
        Case::new(
            "map_shape_key_was_a_value",
            "A shape's key is a string of the table like any other: `name` was written as a value first, so the key is its number.",
        )
        .all_forms(map(vec![
            ("a", text("name")),
            ("b", map(vec![("name", Value::Int(1))])),
        ])),
        Case::new(
            "map_many_keys",
            "A map of 20 keys: a shape of 20, its marker `20 << 1 | 1`.",
        )
        .all_forms(Value::Map(
            (0..20)
                .map(|i| (format!("key{i}"), Value::Int(i)))
                .collect(),
        )),
    ]
}

fn records() -> Vec<Case> {
    let users = Value::List(vec![user("alice", 30), user("bob", 25), user("carol", 41)]);
    let reordered = Value::List(vec![
        user("alice", 30),
        map(vec![("age", Value::Int(25)), ("name", text("bob"))]),
    ]);
    let nested = Value::List(vec![
        map(vec![("who", user("alice", 30)), ("n", Value::Int(1))]),
        map(vec![("who", user("bob", 25)), ("n", Value::Int(2))]),
    ]);
    let empty = || Value::Map(Map::new());
    vec![
        Case::new(
            "records_users",
            "Version 2: a list of maps that all have the same keys in the same order is tag 16: a count, the shape once, then the values of each map with no tag of its own.",
        )
        .all_forms(users),
        Case::new(
            "records_key_order_differs",
            "A map with the keys in another order is another shape, so the list is a plain list (tag 6) of two shaped maps.",
        )
        .all_forms(reordered),
        Case::new("records_single", "A list of one map is records too.")
            .all_forms(Value::List(vec![user("alice", 30)])),
        Case::new(
            "records_nested",
            "Records inside records: the inner shape is written the first time it is met, inside the first record.",
        )
        .all_forms(nested),
        Case::new(
            "records_with_an_empty_map",
            "A list with an empty map in it is not records (a shape has at least one key): a plain list of a map and an empty map (tag 7, count 0).",
        )
        .all_forms(Value::List(vec![user("alice", 30), empty()])),
        Case::new("records_of_empty_maps", "A list of empty maps is a plain list.")
            .all_forms(Value::List(vec![empty(), empty()])),
    ]
}

/// A document of version 1, the value it holds, and the version 2 document it upgrades to.
fn upgrade(name: &str, why: &str, old: &[u8], value: Value) -> Case {
    let upgraded = binary::upgrade(old).expect("version 1 reads");
    assert_eq!(
        upgraded,
        binary::encode(&value),
        "{name}: upgrading writes the value"
    );
    Case::new(name, why)
        .value(value)
        .binary(old.to_vec())
        .binary_v2(upgraded)
}

fn upgrades() -> Vec<Case> {
    let mut all = vec![upgrade_records(), upgrade_stream(), upgrade_packed_lists()];
    all.extend([upgrade_single_floats(), upgrade_strings_and_refs()]);
    all
}

fn upgrade_records() -> Case {
    let mut users = V1Writer::new();
    users.list(2);
    for (name, age) in [("alice", 30), ("bob", 25)] {
        users.map(2).key("name").str(name).key("age").int(age);
    }
    upgrade(
        "upgrade_records",
        "A version 1 document read and written as version 2: two maps with the same keys, each written out in full in version 1, are records in version 2.",
        &users.finish(),
        Value::List(vec![user("alice", 30), user("bob", 25)]),
    )
}

fn upgrade_stream() -> Case {
    let mut body = V1Writer::body();
    for n in 1..=3 {
        body.map(1).key("n").int(n);
    }
    let mut stream = V1Writer::new();
    stream
        .map(2)
        .key("name")
        .str("audit")
        .key("entries")
        .stream(3, &body.finish());
    let entries = (1..=3).map(|n| map(vec![("n", Value::Int(n))])).collect();
    upgrade(
        "upgrade_stream",
        "A version 1 stream (tag 11, its own string table) is read as the list of its items and written as a list: a stream is how a typed writer keeps a list, not a value of its own.",
        &stream.finish(),
        map(vec![
            ("name", text("audit")),
            ("entries", Value::List(entries)),
        ]),
    )
}

fn upgrade_packed_lists() -> Case {
    let mut packed = V1Writer::new();
    packed.map(3);
    packed.key("a").ints(&[1, 2, 3]);
    packed.key("b").floats(&[0.5, 1.5]);
    packed.key("c").floats32(&[0.25]);
    upgrade(
        "upgrade_packed_lists",
        "Packed lists of version 1 (tags 10, 13, 14) read as lists and are packed again the way version 2 chooses: the f32 list stays narrow, the others as before.",
        &packed.finish(),
        map(vec![
            ("a", ints(&[1, 2, 3])),
            ("b", floats(&[0.5, 1.5])),
            ("c", floats(&[0.25])),
        ]),
    )
}

fn upgrade_single_floats() -> Case {
    let mut single = V1Writer::new();
    single.list(2).float32(0.5).float(0.5);
    upgrade(
        "upgrade_single_floats",
        "A version 1 float of four bytes (tag 12) and the same number as eight (tag 4) read as the same float, and are written alike (four bytes) in version 2.",
        &single.finish(),
        floats(&[0.5, 0.5]),
    )
}

fn upgrade_strings_and_refs() -> Case {
    let mut mixed = V1Writer::new();
    mixed
        .map(3)
        .key("logo")
        .reference("file", "images/logo.png");
    mixed.key("n").int(-1000);
    mixed.key("name").str("file");
    let logo = Value::Ref(Ref::parse("file", "images/logo.png").expect("a reference"));
    upgrade(
        "upgrade_strings_and_refs",
        "Strings, references and ints keep their meaning: the string table is rebuilt in the order version 2 writes first uses, and a small int is one byte.",
        &mixed.finish(),
        map(vec![
            ("logo", logo),
            ("n", Value::Int(-1000)),
            ("name", text("file")),
        ]),
    )
}
