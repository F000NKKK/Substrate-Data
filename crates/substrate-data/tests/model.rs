//! The model: values, borrowed values, paths and maps.

use substrate_data::{Bump, Map, MapRef, Path, Ref, Seg, Value, ValueKind, ValueRef};

fn file_ref() -> Ref {
    Ref::parse("file", "x").expect("a reference")
}

fn sample() -> Value {
    let mut inner = Map::new();
    inner.insert("n", Value::Int(3));
    let mut m = Map::new();
    m.insert("name", Value::from("a"));
    m.insert(
        "list",
        Value::List(vec![Value::Bool(true), Value::Map(inner)]),
    );
    Value::Map(m)
}

#[test]
fn a_value_says_which_case_it_is() {
    let cases = [
        (Value::Null, ValueKind::Null, "null"),
        (Value::from(true), ValueKind::Bool, "true/false"),
        (Value::from(4i64), ValueKind::Int, "an integer"),
        (Value::from(1.5f64), ValueKind::Float, "a number"),
        (Value::from("s"), ValueKind::Str, "a string"),
        (Value::from(vec![Value::Null]), ValueKind::List, "a list"),
        (Value::from(Map::new()), ValueKind::Map, "an object"),
        (Value::Ref(file_ref()), ValueKind::Ref, "a reference"),
        (Value::Bytes(vec![1]), ValueKind::Bytes, "bytes"),
    ];
    for (v, kind, word) in cases {
        assert_eq!(v.kind(), kind);
        assert_eq!(kind.to_string(), word);
    }
    assert_eq!(Value::default(), Value::Null);
}

#[test]
fn a_value_gives_its_number_back_only_when_it_is_one() {
    let ints = [
        (Value::Int(7), Some(7)),
        (Value::Float(2.0), Some(2)),
        (Value::Float(2.5), None),
        (Value::from("x"), None),
    ];
    for (v, want) in ints {
        assert_eq!(v.as_i64(), want);
    }
    let floats = [
        (Value::Int(2), Some(2.0)),
        (Value::Float(2.5), Some(2.5)),
        (Value::Null, None),
    ];
    for (v, want) in floats {
        assert_eq!(v.as_f64(), want);
    }
}

#[test]
fn a_value_gives_its_content_back_only_when_it_is_that_case() {
    assert!(Value::Null.is_null() && !Value::Bool(false).is_null());
    assert_eq!(Value::Bool(true).as_bool(), Some(true));
    assert_eq!(Value::Int(1).as_bool(), None);
    assert_eq!(Value::from("x").as_str(), Some("x"));
    assert_eq!(Value::Null.as_str(), None);
    assert_eq!(Value::List(vec![]).as_list(), Some(&[][..]));
    assert_eq!(Value::Null.as_list(), None);
    assert!(Value::Map(Map::new()).as_map().is_some());
    assert!(Value::Null.as_map().is_none());
}

#[test]
fn values_are_found_by_path() {
    let v = sample();
    assert_eq!(v.get("name"), Some(&Value::from("a")));
    assert_eq!(Value::Null.get("name"), None);
    let path = Path::root().key("list").index(1).key("n");
    assert_eq!(v.at(&path), Some(&Value::Int(3)));
    assert_eq!(v.at(&Path::root().key("list").index(9)), None);
    assert_eq!(v.at(&Path::root().key("name").index(0)), None);
    assert_eq!(v.at(&Path::root().key("missing")), None);
    assert_eq!(v.at(&Path::root()), Some(&sample()));
}

#[test]
fn values_are_changed_by_steps_that_lead_somewhere() {
    let mut v = sample();
    let steps = [Seg::Key("list".into()), Seg::Index(0)];
    *v.at_mut(&steps).expect("a bool") = Value::Bool(false);
    assert_eq!(
        v.at(&Path::root().key("list").index(0)),
        Some(&Value::Bool(false))
    );
}

#[test]
fn steps_that_lead_nowhere_change_nothing() {
    let mut v = sample();
    let wrong: [&[Seg]; 4] = [
        &[Seg::Index(0)],
        &[Seg::Key("list".into()), Seg::Key("x".into())],
        &[Seg::Key("nope".into())],
        &[Seg::Key("list".into()), Seg::Index(5)],
    ];
    for steps in wrong {
        assert!(v.at_mut(steps).is_none(), "{steps:?}");
    }
    assert_eq!(v, sample());
}

#[test]
fn paths_are_built_and_displayed() {
    let mut p = Path::root();
    assert!(p.is_root());
    assert_eq!(p.to_string(), "(root)");
    p.push(Seg::Key("tasks".into()));
    p.push(Seg::Index(2));
    p.push(Seg::Key("two words".into()));
    p.push(Seg::Key("owner".into()));
    assert!(!p.is_root());
    assert_eq!(p.to_string(), "tasks[2][\"two words\"].owner");
    assert_eq!(p.segments().len(), 4);
    p.pop();
    let rest = Path::root().key("a").index(0);
    assert_eq!(p.then(&rest).to_string(), "tasks[2][\"two words\"].a[0]");
}

#[test]
fn keys_that_are_not_plain_names_are_quoted() {
    assert_eq!(Path::root().key("9lives").to_string(), "[\"9lives\"]");
    assert_eq!(Path::root().key("$note").to_string(), "$note");
    assert_eq!(Path::root().key("").to_string(), "[\"\"]");
}

#[test]
fn a_path_reads_and_writes_as_its_steps() {
    let p = Path::root().key("a").index(1);
    let v = substrate_data::Schema::write(&p);
    assert_eq!(substrate_data::read_value::<Path>(&v).expect("a path"), p);
    let frame = substrate_data::frame::to_frame(&p);
    assert_eq!(substrate_data::frame::read_frame::<Path>(&frame), Some(p));
}

fn rich() -> Value {
    let mut v = sample();
    if let Value::Map(m) = &mut v {
        m.insert("ref", Value::Ref(file_ref()));
        m.insert("bytes", Value::Bytes(vec![1, 2]));
        m.insert("f", Value::Float(1.5));
        m.insert("nil", Value::Null);
    }
    v
}

#[test]
fn a_borrowed_value_copies_back_to_the_owned_one() {
    let bump = Bump::new();
    let v = rich();
    let b = v.borrow(&bump);
    assert_eq!(b.to_value(), v);
    assert_eq!(b.kind(), ValueKind::Map);
    assert_eq!(b.get("name").and_then(ValueRef::as_str), Some("a"));
    assert_eq!(b.get("missing"), None);
    assert_eq!(ValueRef::Null.get("x"), None);
}

#[test]
fn a_borrowed_map_lists_its_entries() {
    let bump = Bump::new();
    let v = rich();
    let map = v.borrow(&bump).as_map().expect("a map");
    assert_eq!(map.len(), 6);
    assert!(!map.is_empty() && MapRef::default().is_empty());
    assert_eq!(map.keys().next(), Some("name"));
    assert_eq!(map.iter().count(), 6);
}

#[test]
fn a_borrowed_value_knows_each_case() {
    let bump = Bump::new();
    let v = rich();
    let b = v.borrow(&bump);
    let kinds = [
        ("list", ValueKind::List),
        ("ref", ValueKind::Ref),
        ("bytes", ValueKind::Bytes),
        ("f", ValueKind::Float),
        ("nil", ValueKind::Null),
        ("name", ValueKind::Str),
    ];
    for (key, kind) in kinds {
        assert_eq!(b.get(key).map(ValueRef::kind), Some(kind), "{key}");
    }
    assert_eq!(ValueRef::Bool(true).kind(), ValueKind::Bool);
    assert_eq!(ValueRef::Int(1).kind(), ValueKind::Int);
    assert_eq!(b.get("ref").and_then(|r| r.to_ref()), Some(file_ref()));
    assert_eq!(ValueRef::Null.to_ref(), None);
}

#[test]
fn a_borrowed_value_gives_its_content_back_only_when_it_is_that_case() {
    let numbers = [
        (ValueRef::Int(3), Some(3), Some(3.0)),
        (ValueRef::Float(4.0), Some(4), Some(4.0)),
        (ValueRef::Float(0.5), None, Some(0.5)),
        (ValueRef::Str("x"), None, None),
    ];
    for (v, int, float) in numbers {
        assert_eq!((v.as_i64(), v.as_f64()), (int, float), "{v:?}");
    }
    assert_eq!(ValueRef::Bool(true).as_bool(), Some(true));
    assert_eq!(ValueRef::Int(1).as_bool(), None);
    assert_eq!(ValueRef::Null.as_str(), None);
    assert!(ValueRef::Null.as_list().is_none() && ValueRef::Null.as_map().is_none());
    assert!(ValueRef::Null.is_null() && !ValueRef::Bool(true).is_null());
}
