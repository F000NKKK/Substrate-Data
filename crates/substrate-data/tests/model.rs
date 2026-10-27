//! The model: values, borrowed values, paths, names and typed spellings.

use substrate_data::typed::{self, Spelled, TypedError};
use substrate_data::{
    Atom, Bump, KindName, Map, Path, Ref, RefKey, Seg, Value, ValueKind, ValueRef, names,
};

substrate_data::kind!(File = "file");

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
fn a_value_says_which_case_it_is_and_gives_it_back() {
    let cases = [
        (Value::Null, ValueKind::Null),
        (Value::from(true), ValueKind::Bool),
        (Value::from(4i64), ValueKind::Int),
        (Value::from(1.5f64), ValueKind::Float),
        (Value::from("s"), ValueKind::Str),
        (Value::from(vec![Value::Null]), ValueKind::List),
        (Value::from(Map::new()), ValueKind::Map),
        (Value::Ref(Ref::parse("file", "x").unwrap()), ValueKind::Ref),
        (Value::Bytes(vec![1]), ValueKind::Bytes),
    ];
    let words = [
        "null",
        "true/false",
        "an integer",
        "a number",
        "a string",
        "a list",
        "an object",
        "a reference",
        "bytes",
    ];
    for ((v, kind), word) in cases.iter().zip(words) {
        assert_eq!(v.kind(), *kind);
        assert_eq!(kind.to_string(), word);
    }
    assert!(Value::Null.is_null() && !Value::Bool(false).is_null());
    assert_eq!(Value::Bool(true).as_bool(), Some(true));
    assert_eq!(Value::Int(1).as_bool(), None);
    assert_eq!(Value::Int(7).as_i64(), Some(7));
    assert_eq!(Value::Float(2.0).as_i64(), Some(2));
    assert_eq!(Value::Float(2.5).as_i64(), None);
    assert_eq!(Value::Str("x".into()).as_i64(), None);
    assert_eq!(Value::Int(2).as_f64(), Some(2.0));
    assert_eq!(Value::Float(2.5).as_f64(), Some(2.5));
    assert_eq!(Value::Null.as_f64(), None);
    assert_eq!(Value::from("x").as_str(), Some("x"));
    assert_eq!(Value::Null.as_str(), None);
    assert_eq!(Value::List(vec![]).as_list(), Some(&[][..]));
    assert_eq!(Value::Null.as_list(), None);
    assert!(Value::Map(Map::new()).as_map().is_some());
    assert!(Value::Null.as_map().is_none());
    assert_eq!(Value::default(), Value::Null);
}

#[test]
fn values_are_found_and_changed_by_path() {
    let mut v = sample();
    assert_eq!(v.get("name"), Some(&Value::from("a")));
    assert_eq!(Value::Null.get("name"), None);
    let path = Path::root().key("list").index(1).key("n");
    assert_eq!(v.at(&path), Some(&Value::Int(3)));
    assert_eq!(v.at(&Path::root().key("list").index(9)), None);
    assert_eq!(v.at(&Path::root().key("name").index(0)), None);
    assert_eq!(v.at(&Path::root().key("missing")), None);
    assert_eq!(v.at(&Path::root()), Some(&sample()));

    let segs = [Seg::Key("list".into()), Seg::Index(0)];
    *v.at_mut(&segs).unwrap() = Value::Bool(false);
    assert_eq!(
        v.at(&Path::root().key("list").index(0)),
        Some(&Value::Bool(false))
    );
    assert!(v.at_mut(&[Seg::Index(0)]).is_none());
    assert!(
        v.at_mut(&[Seg::Key("list".into()), Seg::Key("x".into())])
            .is_none()
    );
    assert!(v.at_mut(&[Seg::Key("nope".into())]).is_none());
    assert!(
        v.at_mut(&[Seg::Key("list".into()), Seg::Index(5)])
            .is_none()
    );
}

#[test]
fn paths_are_built_displayed_and_joined() {
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
    assert_eq!(Path::root().key("9lives").to_string(), "[\"9lives\"]");
    assert_eq!(Path::root().key("$note").to_string(), "$note");
    assert_eq!(Path::root().key("").to_string(), "[\"\"]");
}

#[test]
fn a_path_reads_and_writes_as_its_steps() {
    let p = Path::root().key("a").index(1);
    let v = substrate_data::Schema::write(&p);
    assert_eq!(substrate_data::read_value::<Path>(&v).unwrap(), p);
    let frame = substrate_data::frame::to_frame(&p);
    assert_eq!(substrate_data::frame::read_frame::<Path>(&frame), Some(p));
}

#[test]
fn a_map_keeps_order_and_indexes_when_big() {
    let mut m = Map::new();
    assert!(m.is_empty());
    assert_eq!(m.insert("a", Value::Int(1)), None);
    assert_eq!(m.insert("a", Value::Int(2)), Some(Value::Int(1)));
    assert_eq!(m.len(), 1);
    for i in 0..40 {
        m.insert(format!("k{i}"), Value::Int(i));
    }
    assert_eq!(m.get("k39"), Some(&Value::Int(39)));
    assert_eq!(m.index_of("k0"), Some(1));
    assert_eq!(m.value_at(0), Some(&Value::Int(2)));
    assert_eq!(m.value_at(99), None);
    assert_eq!(m.remove("k0"), Some(Value::Int(0)));
    assert_eq!(m.remove("k0"), None);
    assert_eq!(m.index_of("k1"), Some(1));
    *m.get_mut("k5").unwrap() = Value::Null;
    assert_eq!(m.get("k5"), Some(&Value::Null));
    m.retain(|k, _| k == "a" || k == "k7");
    assert_eq!(m.keys().collect::<Vec<_>>(), ["a", "k7"]);
    assert_eq!(m.values().count(), 2);
    assert_eq!(m.get("k7"), Some(&Value::Int(7)));
    assert!(format!("{m:?}").starts_with("Map("));
    let borrowed: Vec<_> = (&m).into_iter().map(|(k, _)| k.to_owned()).collect();
    let owned: Vec<_> = m.clone().into_iter().map(|(k, _)| k).collect();
    assert_eq!(borrowed, owned);
    let collected: Map = [("x", Value::Null)].into_iter().collect();
    assert_eq!(collected.len(), 1);
}

#[test]
fn borrowed_values_mirror_owned_ones() {
    let bump = Bump::new();
    let mut v = sample();
    if let Value::Map(m) = &mut v {
        m.insert("ref", Value::Ref(Ref::parse("file", "x").unwrap()));
        m.insert("bytes", Value::Bytes(vec![1, 2]));
        m.insert("f", Value::Float(1.5));
        m.insert("nil", Value::Null);
    }
    let b = v.borrow(&bump);
    assert_eq!(b.to_value(), v);
    assert_eq!(b.kind(), ValueKind::Map);
    let map = b.as_map().unwrap();
    assert_eq!(map.len(), 6);
    assert!(!map.is_empty());
    assert!(MapRef_default().is_empty());
    assert_eq!(map.keys().next(), Some("name"));
    assert_eq!(map.iter().count(), 6);
    assert_eq!(b.get("name").and_then(|n| n.as_str()), Some("a"));
    assert_eq!(b.get("missing"), None);
    assert_eq!(ValueRef::Null.get("x"), None);
    let list = b.get("list").unwrap().as_list().unwrap();
    assert_eq!(list[0].as_bool(), Some(true));
    assert_eq!(ValueRef::Int(1).as_bool(), None);
    assert_eq!(ValueRef::Int(3).as_i64(), Some(3));
    assert_eq!(ValueRef::Float(4.0).as_i64(), Some(4));
    assert_eq!(ValueRef::Str("x").as_i64(), None);
    assert_eq!(ValueRef::Int(3).as_f64(), Some(3.0));
    assert_eq!(ValueRef::Float(0.5).as_f64(), Some(0.5));
    assert_eq!(ValueRef::Null.as_f64(), None);
    assert_eq!(ValueRef::Null.as_str(), None);
    assert!(ValueRef::Null.as_list().is_none() && ValueRef::Null.as_map().is_none());
    assert!(ValueRef::Null.is_null() && !ValueRef::Bool(true).is_null());
    let r = b.get("ref").unwrap();
    assert_eq!(r.to_ref(), Some(Ref::parse("file", "x").unwrap()));
    assert_eq!(ValueRef::Null.to_ref(), None);
    assert_eq!(r.kind(), ValueKind::Ref);
    assert_eq!(b.get("bytes").unwrap().kind(), ValueKind::Bytes);
    assert_eq!(b.get("f").unwrap().kind(), ValueKind::Float);
    assert_eq!(b.get("nil").unwrap().kind(), ValueKind::Null);
    assert_eq!(list[1].kind(), ValueKind::Map);
    assert_eq!(ValueRef::Bool(true).kind(), ValueKind::Bool);
    assert_eq!(ValueRef::Int(1).kind(), ValueKind::Int);
    assert_eq!(ValueRef::Str("").kind(), ValueKind::Str);
    assert_eq!(b.get("list").unwrap().kind(), ValueKind::List);
}

#[expect(
    non_snake_case,
    reason = "reads as the type's default in the assertion"
)]
fn MapRef_default() -> substrate_data::MapRef<'static> {
    substrate_data::MapRef::default()
}

#[test]
fn names_and_keys_are_checked_once() {
    assert!(KindName::new("user_2").is_ok());
    for bad in ["", "2x", "Up", "a-b", "a b"] {
        assert_eq!(
            KindName::new(bad),
            Err(TypedError::BadKind { kind: bad.into() })
        );
    }
    let k = KindName::new("doc").unwrap();
    assert_eq!(k, "doc");
    assert_eq!(k, *"doc");
    assert_eq!(format!("{k:?}"), "KindName(\"doc\")");
    assert_eq!(k.to_string(), "doc");
    assert_eq!(k.atom(), Atom::new("doc"));
    assert!(KindName::new("a").unwrap() < KindName::new("b").unwrap());
    assert_eq!(
        KindName::new("a").unwrap().partial_cmp(&k),
        Some(std::cmp::Ordering::Less)
    );

    assert!(RefKey::new("a/b").is_ok());
    for bad in ["", "a b", "a\nb"] {
        assert!(RefKey::new(bad).is_err());
        assert!(!RefKey::is_valid(bad));
    }
    let id = RefKey::new("x").unwrap();
    assert_eq!(id, "x");
    assert_eq!(id, *"x");
    assert_eq!(format!("{id:?}"), "RefKey(\"x\")");
    assert_eq!(id.to_string(), "x");

    let r = Ref::parse("file", "a/b").unwrap();
    assert_eq!(r.to_string(), "@file:a/b");
    assert_eq!(r, Ref::of::<File>(&"a/b".to_owned()));
    assert_eq!(
        Ref::parse("file", "a b"),
        Err(TypedError::BadId {
            kind: Some(KindName::new("file").unwrap()),
            id: "a b".into()
        })
    );
    assert!(Ref::parse("File", "a").is_err());
}

#[test]
fn atoms_convert_compare_and_default() {
    let a = Atom::new("model_test_atom");
    assert_eq!(Atom::from("model_test_atom"), a);
    assert_eq!(Atom::from(String::from("model_test_atom")), a);
    assert_eq!(Atom::from(&String::from("model_test_atom")), a);
    assert_eq!(a, "model_test_atom");
    assert_eq!(a, *"model_test_atom");
    assert_eq!(a, String::from("model_test_atom"));
    assert_eq!(String::from("model_test_atom"), a);
    assert_eq!(a.len(), 15);
    assert_eq!(Atom::default().as_str(), "");
    assert_eq!(format!("{a:?}"), "Atom(\"model_test_atom\")");
}

#[test]
fn typed_spellings_classify_and_flatten() {
    assert_eq!(names::TYPED, '@');
    assert_eq!(names::Word::Type.to_string(), "type");
    assert_eq!(names::Word::Value.as_str(), "value");
    assert_eq!(
        typed::classify("plain").unwrap(),
        Spelled::Plain("plain".into())
    );
    assert_eq!(
        typed::classify("@@at").unwrap(),
        Spelled::Plain("@at".into())
    );
    assert!(matches!(
        typed::classify("@file:x").unwrap(),
        Spelled::Ref(_)
    ));
    assert_eq!(
        typed::classify("@bytes:AQI=").unwrap(),
        Spelled::Bytes(vec![1, 2])
    );
    assert!(matches!(
        typed::classify("@bytes:***"),
        Err(TypedError::BadBytes(_))
    ));
    assert!(matches!(
        typed::classify("@nokind"),
        Err(TypedError::NoKind { .. })
    ));
    assert!(matches!(
        typed::classify("@Bad:x"),
        Err(TypedError::BadKind { .. })
    ));
    assert!(matches!(
        typed::classify("@file:"),
        Err(TypedError::BadId { .. })
    ));

    assert_eq!(Spelled::Plain("@x".into()).kind(), ValueKind::Str);
    assert_eq!(Spelled::Bytes(vec![]).kind(), ValueKind::Bytes);
    let r = Ref::parse("file", "x").unwrap();
    assert_eq!(Spelled::Ref(r.clone()).kind(), ValueKind::Ref);
    assert_eq!(Spelled::Ref(r.clone()).into_value(), Value::Ref(r.clone()));
    assert_eq!(Spelled::Plain("@x".into()).into_value(), Value::from("@@x"));
    assert_eq!(Spelled::Bytes(vec![1]).into_value(), Value::Bytes(vec![1]));

    assert_eq!(
        typed::spelling(&Value::Ref(r.clone())).as_deref(),
        Some("@file:x")
    );
    assert_eq!(
        typed::spelling(&Value::Bytes(vec![1, 2])).as_deref(),
        Some("@bytes:AQI=")
    );
    assert_eq!(typed::spelling(&Value::Int(1)), None);

    let mut m = Map::new();
    m.insert("r", Value::Ref(r));
    let flat = typed::flatten(&Value::List(vec![
        Value::Map(m),
        Value::Bytes(vec![1, 2]),
        Value::Null,
    ]));
    assert_eq!(flat.to_string_for_test(), "[{r:@file:x},@bytes:AQI=,null]");
}

trait Show {
    fn to_string_for_test(&self) -> String;
}

impl Show for Value {
    fn to_string_for_test(&self) -> String {
        match self {
            Value::Null => "null".into(),
            Value::Str(s) => s.clone(),
            Value::List(l) => format!(
                "[{}]",
                l.iter()
                    .map(Show::to_string_for_test)
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Value::Map(m) => format!(
                "{{{}}}",
                m.iter()
                    .map(|(k, v)| format!("{k}:{}", v.to_string_for_test()))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            other => format!("{other:?}"),
        }
    }
}
