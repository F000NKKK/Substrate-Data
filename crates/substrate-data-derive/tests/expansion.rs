//! `#[derive(Schema)]` expanded for every shape of type it supports, and
//! what the expansions do in each form.

use std::collections::BTreeMap;

use substrate_data::frame::{read_frame, read_view, to_frame};
use substrate_data::with::{Plain, Raw, Values};
use substrate_data::{Schema, Shape, Value, from_text, read_bytes, read_value, to_bytes};

fn through_every_form<T: Schema + PartialEq + std::fmt::Debug>(x: &T) {
    assert_eq!(&read_value::<T>(&x.write()).expect("from a value"), x);
    assert_eq!(&read_bytes::<T>(&to_bytes(x)).expect("from bytes"), x);
    assert_eq!(read_frame::<T>(&to_frame(x)).as_ref(), Some(x));
    let text = substrate_data::to_text(x);
    assert_eq!(
        &from_text::<T>(&text).expect("from text").value,
        x,
        "{text}"
    );
}

fn zero() -> u32 {
    10
}

/// A record.
///
/// With a second paragraph.
#[derive(Schema, Debug, Clone, PartialEq)]
#[data(rename_all = "camelCase", view)]
struct Record {
    /// Its name.
    display_name: String,
    #[data(default = "zero", range = "0..=100")]
    level: u32,
    #[data(default)]
    note: Option<String>,
    #[data(skip)]
    cache: Vec<u8>,
    #[data(rename = "kept_as_is")]
    other_name: bool,
    #[data(with = "Raw")]
    payload: Vec<u8>,
    #[data(with = "Values<Plain>")]
    tags: BTreeMap<String, u16>,
}

fn record() -> Record {
    Record {
        display_name: "r".into(),
        level: 50,
        note: Some("n".into()),
        cache: Vec::new(),
        other_name: true,
        payload: vec![1, 2],
        tags: BTreeMap::from([("a".to_owned(), 1)]),
    }
}

#[test]
fn a_struct_reads_writes_and_packs_by_its_attributes() {
    through_every_form(&record());
    let v = record().write();
    assert!(v.get("displayName").is_some() && v.get("kept_as_is").is_some());
    assert!(v.get("cache").is_none());
    let left_out = read_value::<Record>(&Value::Map(
        [
            ("displayName".to_owned(), Value::from("x")),
            ("kept_as_is".to_owned(), Value::Bool(false)),
            ("tags".to_owned(), Value::Map(substrate_data::Map::new())),
        ]
        .into_iter()
        .collect(),
    ))
    .unwrap();
    assert_eq!(
        (left_out.level, left_out.note, left_out.payload),
        (10, None, vec![])
    );
    let mut over = v.clone();
    if let Value::Map(m) = &mut over {
        m.insert("level", Value::Int(101));
    }
    let problems = read_value::<Record>(&over).unwrap_err();
    assert_eq!(problems[0].message, "101 is out of range (0..=100)");
}

#[test]
fn a_struct_describes_its_fields_docs_and_ranges() {
    let Shape::Struct(s) = Record::shape() else {
        panic!("a struct")
    };
    assert_eq!(s.name, "Record");
    assert_eq!(s.doc, "A record.\n\nWith a second paragraph.");
    let names: Vec<_> = s.fields.iter().map(|f| f.name).collect();
    assert_eq!(
        names,
        [
            "displayName",
            "level",
            "note",
            "kept_as_is",
            "payload",
            "tags"
        ]
    );
    assert_eq!(s.fields[0].doc, "Its name.");
    assert!(s.fields[0].required && !s.fields[1].required && !s.fields[2].required);
    assert_eq!(s.fields[1].range, Some((0.0, 100.0)));
    assert!(!s.fields[4].required, "raw bytes default to empty");
}

#[test]
fn a_view_borrows_the_frame() {
    let frame = to_frame(&record());
    let v = read_view::<Record>(&frame).unwrap();
    assert_eq!(v.display_name, "r");
    assert_eq!(v.level, 50);
    assert_eq!(v.note, Some("n"));
    assert_eq!(v.payload, vec![1, 2]);
    assert!(v.other_name);
    assert_eq!(v.tags, BTreeMap::from([("a".to_owned(), 1)]));
}

#[test]
fn the_field_macro_names_fields_as_files_write_them() {
    assert_eq!(substrate_data::field!(Record, display_name), "displayName");
    assert_eq!(substrate_data::field!(Record, other_name), "kept_as_is");
}

/// A name for everyone.
#[derive(Schema, Debug, Clone, Copy, PartialEq)]
enum Visibility {
    /// Anyone.
    Everyone,
    #[data(rename = "team_only")]
    Team,
    NoOne,
}

#[test]
fn an_enum_of_names_is_a_string() {
    for v in [Visibility::Everyone, Visibility::Team, Visibility::NoOne] {
        through_every_form(&v);
    }
    assert_eq!(Visibility::Team.write(), Value::from("team_only"));
    assert_eq!(Visibility::NoOne.write(), Value::from("no_one"));
    let typo = read_value::<Visibility>(&Value::from("teem_only")).unwrap_err();
    assert_eq!(
        typo[0].message,
        "unknown `teem_only` — did you mean `team_only`?"
    );
    let number = read_value::<Visibility>(&Value::Int(1)).unwrap_err();
    assert!(number[0].message.contains("expected a name"));
    let Shape::Enum(e) = Visibility::shape() else {
        panic!("an enum")
    };
    assert!(e.tag.is_none());
    assert_eq!(e.doc, "A name for everyone.");
    assert_eq!(e.variants[0].doc, "Anyone.");
    assert_eq!(
        read_frame::<Visibility>(&to_frame(&Visibility::NoOne)),
        Some(Visibility::NoOne)
    );
}
