//! Names and typed spellings: kinds, reference keys, atoms, and the string
//! forms of references and bytes.

use substrate_data::typed::{self, Spelled, TypedError};
use substrate_data::{Atom, KindName, Map, Ref, RefKey, Value, ValueKind, names};

substrate_data::kind!(File = "file");

fn file_ref() -> Ref {
    Ref::parse("file", "x").expect("a reference")
}

#[test]
fn kind_names_are_checked_once() {
    assert!(KindName::new("user_2").is_ok());
    for bad in ["", "2x", "Up", "a-b", "a b"] {
        assert_eq!(
            KindName::new(bad),
            Err(TypedError::BadKind { kind: bad.into() })
        );
    }
}

#[test]
fn a_kind_name_compares_prints_and_orders() {
    let doc = KindName::new("doc").expect("a kind");
    assert_eq!(doc, "doc");
    assert_eq!(doc, *"doc");
    assert_eq!(format!("{doc:?}"), "KindName(\"doc\")");
    assert_eq!(doc.to_string(), "doc");
    assert_eq!(doc.atom(), Atom::new("doc"));
    let a = KindName::new("a").expect("a kind");
    assert!(a < KindName::new("b").expect("a kind"));
    assert_eq!(a.partial_cmp(&doc), Some(std::cmp::Ordering::Less));
}

#[test]
fn a_reference_key_is_not_empty_and_has_no_blanks() {
    assert!(RefKey::new("a/b").is_ok());
    for bad in ["", "a b", "a\nb"] {
        assert!(RefKey::new(bad).is_err());
        assert!(!RefKey::is_valid(bad));
    }
    let id = RefKey::new("x").expect("a key");
    assert_eq!(id, "x");
    assert_eq!(id, *"x");
    assert_eq!(format!("{id:?}"), "RefKey(\"x\")");
    assert_eq!(id.to_string(), "x");
}

#[test]
fn a_reference_is_checked_where_it_is_made() {
    let r = Ref::parse("file", "a/b").expect("a reference");
    assert_eq!(r.to_string(), "@file:a/b");
    assert_eq!(r, Ref::of::<File>(&"a/b".to_owned()));
    assert_eq!(
        Ref::parse("file", "a b"),
        Err(TypedError::BadId {
            kind: KindName::new("file").ok(),
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
fn the_shared_words_are_written_once() {
    assert_eq!(names::TYPED, '@');
    assert_eq!(names::Word::Type.to_string(), "type");
    assert_eq!(names::Word::Value.as_str(), "value");
}

#[test]
fn strings_classify_as_plain_text_references_or_bytes() {
    let plain = |s: &str| Spelled::Plain(s.into());
    assert_eq!(typed::classify("plain"), Ok(plain("plain")));
    assert_eq!(typed::classify("@@at"), Ok(plain("@at")));
    assert!(matches!(typed::classify("@file:x"), Ok(Spelled::Ref(_))));
    assert_eq!(
        typed::classify("@bytes:AQI="),
        Ok(Spelled::Bytes(vec![1, 2]))
    );
}

/// Which of the typed errors a string gives, by name.
#[derive(Debug, PartialEq)]
struct TypedErrorKind(&'static str);

fn error_kind(s: &str) -> TypedErrorKind {
    TypedErrorKind(match typed::classify(s) {
        Ok(_) => "none",
        Err(TypedError::BadBytes(_)) => "bytes",
        Err(TypedError::NoKind { .. }) => "no kind",
        Err(TypedError::BadKind { .. }) => "bad kind",
        Err(TypedError::BadId { .. }) => "bad id",
    })
}

#[test]
fn strings_that_start_like_typed_values_but_are_not_are_errors() {
    let cases = [
        ("@bytes:***", "bytes"),
        ("@nokind", "no kind"),
        ("@Bad:x", "bad kind"),
        ("@file:", "bad id"),
    ];
    for (s, kind) in cases {
        assert_eq!(error_kind(s), TypedErrorKind(kind), "{s}");
    }
}

#[test]
fn a_spelling_says_its_value_and_turns_into_it() {
    let r = file_ref();
    assert_eq!(Spelled::Plain("@x".into()).kind(), ValueKind::Str);
    assert_eq!(Spelled::Bytes(vec![]).kind(), ValueKind::Bytes);
    assert_eq!(Spelled::Ref(r.clone()).kind(), ValueKind::Ref);
    assert_eq!(Spelled::Ref(r.clone()).into_value(), Value::Ref(r));
    assert_eq!(Spelled::Plain("@x".into()).into_value(), Value::from("@@x"));
    assert_eq!(Spelled::Bytes(vec![1]).into_value(), Value::Bytes(vec![1]));
}

#[test]
fn typed_values_have_a_string_form_and_plain_ones_none() {
    let spell = |v: Value| typed::spelling(&v);
    assert_eq!(spell(Value::Ref(file_ref())).as_deref(), Some("@file:x"));
    assert_eq!(
        spell(Value::Bytes(vec![1, 2])).as_deref(),
        Some("@bytes:AQI=")
    );
    assert_eq!(spell(Value::Int(1)), None);
}

#[test]
fn flattening_turns_typed_values_into_their_strings() {
    let mut m = Map::new();
    m.insert("r", Value::Ref(file_ref()));
    let flat = typed::flatten(&Value::List(vec![
        Value::Map(m),
        Value::Bytes(vec![1, 2]),
        Value::Null,
    ]));
    let mut want = Map::new();
    want.insert("r", Value::from("@file:x"));
    assert_eq!(
        flat,
        Value::List(vec![
            Value::Map(want),
            Value::from("@bytes:AQI="),
            Value::Null
        ])
    );
}
