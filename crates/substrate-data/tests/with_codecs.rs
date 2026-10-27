//! `#[data(with = "…")]` codecs in every form: references kept as plain
//! ids, spelled text, optionals, lists, maps, plain schemas and raw bytes.

use std::collections::{BTreeMap, HashMap};

use substrate_data::frame::{read_frame, to_frame};
use substrate_data::with::{List, Opt, Plain, Raw, RefTo, Spelled, Values};
use substrate_data::{Schema, Value, from_text, read_bytes, read_value, to_bytes, to_text};

substrate_data::kind!(File = "file", User = "user", Doc = "doc");

#[derive(Schema, Debug, Clone, PartialEq)]
struct Codecs {
    #[data(with = "RefTo<File>")]
    file: String,
    #[data(with = "Spelled")]
    spelled: String,
    #[data(with = "Opt<RefTo<User>>")]
    owner: Option<String>,
    #[data(with = "List<RefTo<Doc>>")]
    docs: Vec<String>,
    #[data(with = "Values<RefTo<Doc>>")]
    by_name: BTreeMap<String, String>,
    #[data(with = "Values<Plain>")]
    hashed: HashMap<String, u32>,
    #[data(with = "Plain")]
    count: u32,
    #[data(with = "Raw")]
    payload: Vec<u8>,
    #[data(with = "Opt<Plain>", default)]
    note: Option<String>,
}

fn sample() -> Codecs {
    Codecs {
        file: "images/logo.png".into(),
        spelled: "@user:alice".into(),
        owner: Some("alice".into()),
        docs: vec!["a".into(), "b".into()],
        by_name: BTreeMap::from([("first".to_owned(), "a".to_owned())]),
        hashed: HashMap::from([("n".to_owned(), 4)]),
        count: 7,
        payload: vec![0, 1, 2],
        note: None,
    }
}

#[test]
fn codec_fields_round_trip_in_every_form() {
    let x = sample();
    assert_eq!(read_value::<Codecs>(&x.write()).unwrap(), x);
    let text = to_text(&x);
    assert_eq!(
        text,
        substrate_data::text::to_string_pretty(&x.write()),
        "straight"
    );
    assert_eq!(from_text::<Codecs>(&text).unwrap().value, x, "{text}");
    assert_eq!(read_bytes::<Codecs>(&to_bytes(&x)).unwrap(), x);
    assert_eq!(read_frame::<Codecs>(&to_frame(&x)), Some(x));
}

#[test]
fn a_codec_field_names_its_problems_with_their_paths() {
    let mut v = sample().write();
    let Value::Map(m) = &mut v else { panic!() };
    m.insert("spelled", Value::Int(3));
    m.insert("docs", Value::from("x"));
    m.insert("by_name", Value::Int(1));
    m.insert("payload", Value::Int(1));
    let problems: Vec<String> = read_value::<Codecs>(&v)
        .unwrap_err()
        .iter()
        .map(|d| [d.path.to_string(), d.message.clone()].join(": "))
        .collect();
    assert_eq!(
        problems,
        [
            "spelled: expected text, found 3",
            "docs: expected a list, found \"x\"",
            "by_name: expected an object, found 1",
            "payload: expected bytes, found 1",
        ]
    );
}

#[test]
fn list_and_map_items_report_their_own_places() {
    let mut v = sample().write();
    let Value::Map(m) = &mut v else { panic!() };
    m.insert(
        "docs",
        Value::List(vec![Value::from("@doc:a"), Value::from("@file:b")]),
    );
    m.insert(
        "by_name",
        Value::Map([("k".to_owned(), Value::Int(1))].into_iter().collect()),
    );
    m.insert(
        "hashed",
        Value::Map([("n".to_owned(), Value::from("x"))].into_iter().collect()),
    );
    let problems: Vec<String> = read_value::<Codecs>(&v)
        .unwrap_err()
        .iter()
        .map(|d| d.path.to_string())
        .collect();
    assert_eq!(problems, ["docs[1]", "by_name.k", "hashed.n"]);
}

#[test]
fn raw_bytes_read_from_text_and_reject_other_text() {
    let mut v = sample().write();
    let Value::Map(m) = &mut v else { panic!() };
    m.insert("payload", Value::from("@bytes:AAEC"));
    assert_eq!(read_value::<Codecs>(&v).unwrap().payload, [0, 1, 2]);
    for bad in ["plain", "@bytes:***", "@file:x"] {
        let Value::Map(entries) = &mut v else {
            panic!()
        };
        entries.insert("payload", Value::from(bad));
        let problems = read_value::<Codecs>(&v).unwrap_err();
        assert_eq!(problems[0].path.to_string(), "payload", "{bad}");
    }
}

#[test]
fn spelled_text_keeps_typed_values_as_their_spelling() {
    let mut v = sample().write();
    let Value::Map(m) = &mut v else { panic!() };
    m.insert(
        "spelled",
        Value::Ref(substrate_data::Ref::parse("file", "x").unwrap()),
    );
    assert_eq!(read_value::<Codecs>(&v).unwrap().spelled, "@file:x");
}

#[test]
fn missing_codec_fields_are_required_unless_they_have_a_default() {
    let mut v = sample().write();
    let Value::Map(m) = &mut v else { panic!() };
    m.remove("file");
    m.remove("owner");
    m.remove("note");
    m.remove("payload");
    let problems: Vec<String> = read_value::<Codecs>(&v)
        .unwrap_err()
        .iter()
        .map(|d| d.message.clone())
        .collect();
    assert_eq!(problems, ["missing `file`"]);
}

#[test]
fn malformed_codec_frames_and_binary_are_errors_not_panics() {
    let x = sample();
    let frame = to_frame(&x);
    for cut in 0..frame.len() {
        assert_eq!(read_frame::<Codecs>(&frame[..cut]), None, "cut at {cut}");
    }
    let bytes = to_bytes(&x);
    for cut in 0..bytes.len() {
        assert!(read_bytes::<Codecs>(&bytes[..cut]).is_err(), "cut at {cut}");
    }
}
