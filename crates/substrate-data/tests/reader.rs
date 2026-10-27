//! The reader and the problems it reports: unknown keys and names with
//! their hints, ranges, optional and default fields, resolvers, and the
//! words a problem uses.

use substrate_data::schema::{Expected, Numbers, field_names, is_metadata_key, read_both};
use substrate_data::{Bump, Path, Problem, Reader, Schema, Seg, Value, ValueRef, read_value};

substrate_data::kind!(File = "file", Doc = "doc", User = "user": u64);

#[derive(Schema, Debug, PartialEq)]
struct Account {
    name: String,
    #[data(default)]
    nickname: Option<String>,
    #[data(default = "three")]
    seats: u8,
}

fn three() -> u8 {
    3
}

#[derive(Schema, Debug, PartialEq)]
struct Contact {
    email: String,
}

#[derive(Schema, Debug, PartialEq)]
struct Empty {}

fn map(entries: &[(&str, Value)]) -> Value {
    Value::Map(
        entries
            .iter()
            .map(|(k, v)| ((*k).to_owned(), v.clone()))
            .collect(),
    )
}

fn messages<T: Schema + std::fmt::Debug>(v: &Value) -> Vec<String> {
    read_value::<T>(v)
        .expect_err("a value that doesn't read")
        .iter()
        .map(|d| [d.path.to_string(), d.message.clone()].join(": "))
        .collect()
}

#[test]
fn unknown_keys_suggest_the_closest_name_or_list_the_known_ones() {
    let v = map(&[
        ("name", "a".into()),
        ("nicknme", "b".into()),
        ("zzzzzzzzzz", Value::Null),
        ("$note", "metadata is fine".into()),
        ("_private", Value::Null),
        ("comment", "so is this".into()),
    ]);
    assert_eq!(
        messages::<Account>(&v),
        [
            "nicknme: unknown field `nicknme` — did you mean `nickname`?",
            "zzzzzzzzzz: unknown field `zzzzzzzzzz` (expected `name`, `nickname`, `seats`)",
        ]
    );
    assert_eq!(
        messages::<Empty>(&map(&[("x", Value::Null)])),
        ["x: unknown field `x`"]
    );
    assert!(read_value::<Empty>(&map(&[])).is_ok());
    assert!(is_metadata_key("$a") && is_metadata_key("_a") && is_metadata_key("comment"));
    assert!(!is_metadata_key("name"));
}

#[test]
fn defaults_fill_the_fields_that_are_left_out() {
    let a = read_value::<Account>(&map(&[("name", "n".into())])).unwrap();
    assert_eq!(
        a,
        Account {
            name: "n".into(),
            nickname: None,
            seats: 3
        }
    );
    assert_eq!(
        messages::<Account>(&map(&[("seats", Value::Int(1))])),
        ["(root): missing `name`"]
    );
    assert_eq!(
        messages::<Account>(&Value::Int(1)),
        ["(root): expected an object, found 1"]
    );
}

#[test]
fn two_structs_read_from_one_object() {
    let v = map(&[
        ("name", "n".into()),
        ("email", "e".into()),
        ("extra", Value::Null),
    ]);
    let arena = Bump::new();
    let borrowed = v.borrow(&arena);
    let mut cx = Reader::new();
    let (a, c) = read_both::<Account, Contact>(&borrowed, &mut cx, &["extra"]).unwrap();
    assert_eq!((a.name.as_str(), c.email.as_str()), ("n", "e"));
    assert!(cx.problems.is_empty());
    let mut strict = Reader::new();
    drop(read_both::<Account, Contact>(&borrowed, &mut strict, &[]));
    assert_eq!(strict.problems.len(), 2);
    assert_eq!(
        field_names(&Account::shape()),
        ["name", "nickname", "seats"]
    );
    assert!(field_names(&String::shape()).is_empty());
}

#[test]
fn field_names_are_checked_when_they_are_used() {
    assert_eq!(substrate_data::field!(Account, seats), "seats");
}

#[test]
#[should_panic(expected = "no such field")]
fn a_field_that_does_not_exist_panics_where_it_is_a_compile_error() {
    assert!(substrate_data::schema::field_name(&[("a", "a")], "b").is_empty());
}

#[test]
fn a_reader_reports_at_the_path_it_is_in() {
    let mut cx = Reader::new();
    assert!(cx.path().is_root());
    let nested = cx.at(Seg::Key("a".into()), |cx| {
        cx.at(Seg::Index(2), |cx| {
            cx.problem("deep");
            cx.problem_of(Problem::BadId, "coded");
            cx.expected(Expected::Bytes, &ValueRef::Int(1));
            cx.path()
        })
    });
    assert_eq!(nested, Path::root().key("a").index(2));
    assert_eq!(cx.problems.len(), 3);
    assert_eq!(cx.problems[1].code, Some(Problem::BadId.into()));
    assert!(format!("{cx:?}").contains("Reader"));
    cx.unknown_name("blu", &["red", "green", "blue"]);
    cx.unknown_name("zzz", &["red"]);
    assert_eq!(
        cx.problems[3].message,
        "unknown `blu` — did you mean `blue`?"
    );
    assert_eq!(cx.problems[4].message, "unknown `zzz` (expected `red`)");
    let arena = Bump::new();
    assert!(cx.map(&Value::Int(1).borrow(&arena)).is_none());
    assert!(cx.map(&map(&[]).borrow(&arena)).is_some());
}

#[test]
fn found_values_are_shown_short_in_a_problem() {
    let long = "x".repeat(60);
    let cases = [
        (Value::from("short"), "found \"short\""),
        (Value::from(long), "found a string"),
        (Value::Float(1.5), "found 1.5"),
        (Value::Bool(true), "found true"),
        (Value::List(vec![]), "found a list"),
        (Value::Null, "found null"),
    ];
    for (v, want) in cases {
        let in_object = messages::<Account>(&Value::List(vec![v.clone()]));
        assert!(in_object[0].contains("an object"), "{in_object:?}");
        let as_number = messages::<u8>(&v);
        assert!(as_number[0].contains(want), "{v:?}: {as_number:?}");
    }
}

#[test]
fn what_a_field_takes_is_said_in_words() {
    let words = [
        (Expected::Bool, "true or false"),
        (Expected::Integer, "an integer"),
        (Expected::Unsigned, "an unsigned integer"),
        (Expected::Number, "a number"),
        (Expected::Str, "a string"),
        (Expected::Text, "text"),
        (Expected::List, "a list"),
        (Expected::Object, "an object"),
        (Expected::OkOrErr, "{ ok } or { err }"),
        (Expected::Reference, "a reference `@kind:id`"),
        (Expected::Bytes, "bytes"),
        (Expected::Name, "a name"),
        (Expected::Uid, "32 hex digits"),
        (Expected::Items(3), "a list of 3"),
    ];
    for (e, w) in words {
        assert_eq!(e.to_string(), w);
    }
}

#[test]
fn ranges_report_the_first_number_out_of_them() {
    let mut cx = Reader::new();
    assert_eq!(cx.in_range("k", Some(5u8), 1.0, 9.0), Some(5));
    assert_eq!(cx.in_range::<u8>("k", None, 1.0, 9.0), None);
    assert_eq!(cx.in_range("k", Some(vec![1i16, 20]), 1.0, 9.0), None);
    assert_eq!(cx.in_range("k", Some([1.5f32, 2.5]), 0.0, 2.0), None);
    assert_eq!(cx.in_range("k", Some(Some(-1i32)), 0.0, 2.0), None);
    assert_eq!(cx.in_range("k", Some(1u64 << 40), 0.0, 2.0), None);
    assert_eq!(cx.in_range("k", Some(7usize), 0.0, 2.5), None);
    let said: Vec<_> = cx.problems.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(
        said,
        [
            "20 is out of range (1..=9)",
            "2.5 is out of range (0..=2)",
            "-1 is out of range (0..=2)",
            "1099511627776 is out of range (0..=2)",
            "7 is out of range (0..=2.5)",
        ]
    );
    assert!(
        cx.problems
            .iter()
            .all(|d| d.code == Some(Problem::OutOfRange.into()))
    );
    let mut all = Vec::new();
    for n in &[1i8, -1] {
        n.numbers(&mut all);
    }
    3i64.numbers(&mut all);
    assert_eq!(all, [1.0, -1.0, 3.0]);
}
