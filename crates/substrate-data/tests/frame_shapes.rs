//! Frames read by their shape alone, as a value: every kind of shape, and
//! frames that are cut or say the wrong thing.

use substrate_data::frame::{Packer, read_frame, read_shaped, to_frame};
use substrate_data::{Atom, Id, Ref, Schema, Uid, Value};

substrate_data::kind!(User = "user": u64, Doc = "doc");

#[derive(Schema, Debug, Clone, PartialEq)]
enum Mood {
    Calm,
    Eager,
}

#[derive(Schema, Debug, Clone, PartialEq)]
#[data(tag = "type")]
enum Event {
    Opened { at: u32 },
    Closed,
}

#[derive(Schema, Debug, Clone, PartialEq)]
struct Entry {
    title: String,
    weight: f32,
}

#[derive(Schema, Debug, Clone, PartialEq)]
struct Shaped {
    flag: bool,
    count: i16,
    total: u64,
    ratio: f32,
    precise: f64,
    name: String,
    nick: Atom,
    unique: Uid,
    owner: Id<User>,
    doc: Id<Doc>,
    anything: Ref,
    mood: Mood,
    event: Event,
    maybe: Option<u8>,
    nothing: Option<u8>,
    signed: Vec<i32>,
    unsigned: Vec<u16>,
    narrow: Vec<f32>,
    wide: Vec<f64>,
    entries: Vec<Entry>,
    any: Value,
}

fn shaped() -> Shaped {
    Shaped {
        flag: true,
        count: -300,
        total: u64::MAX,
        ratio: 0.25,
        precise: 0.1,
        name: "@@kept".into(),
        nick: Atom::new("frame_nick"),
        unique: Uid(5),
        owner: Id::new(77u64),
        doc: Id::new("notes"),
        anything: Ref::parse("file", "a/b").expect("a reference"),
        mood: Mood::Eager,
        event: Event::Opened { at: 9 },
        maybe: Some(2),
        nothing: None,
        signed: vec![-5, 0, 70_000],
        unsigned: vec![1, 2, 65_535],
        narrow: vec![0.5, 1.5],
        wide: vec![0.1, 0.2],
        entries: vec![Entry {
            title: "t".into(),
            weight: 2.0,
        }],
        any: Value::List(vec![Value::Int(1), Value::from("x")]),
    }
}

#[test]
fn a_frame_is_read_by_its_shape_alone() {
    let x = shaped();
    let frame = to_frame(&x);
    assert_eq!(read_frame::<Shaped>(&frame), Some(x.clone()));
    let seen = read_shaped(&frame, &Shaped::shape()).unwrap();
    assert_eq!(seen.get("count"), Some(&Value::Int(-300)));
    assert_eq!(seen.get("name"), Some(&Value::from("@@@kept")));
    assert_eq!(seen.get("mood"), Some(&Value::from("eager")));
    assert_eq!(seen.get("nothing"), Some(&Value::Null));
    assert_eq!(
        seen.get("owner").map(Value::kind),
        Some(substrate_data::ValueKind::Ref)
    );
    assert_eq!(substrate_data::read_value::<Shaped>(&seen).unwrap(), x);
}

#[test]
fn a_frame_that_is_cut_or_lies_is_none_under_every_reading() {
    let frame = to_frame(&shaped());
    let shape = Shaped::shape();
    for cut in 0..frame.len() {
        assert_eq!(read_frame::<Shaped>(&frame[..cut]), None, "cut at {cut}");
        assert_eq!(
            read_shaped(&frame[..cut], &shape),
            None,
            "shaped, cut at {cut}"
        );
    }
    let mut longer = frame.clone();
    longer.insert(0, 0);
    assert_eq!(read_frame::<Shaped>(&longer), None);
    assert_eq!(read_shaped(&[], &shape), None);
}

fn frame_of_uint(n: u64) -> Vec<u8> {
    let mut p = Packer::new();
    p.put_u(n);
    p.finish()
}

fn frame_of_str(s: &str) -> Vec<u8> {
    let mut p = Packer::new();
    p.put_str(s);
    p.finish()
}

#[test]
fn single_values_that_are_not_what_the_shape_says_are_none() {
    let bad_bool = frame_of_uint(2);
    assert_eq!(read_frame::<bool>(&bad_bool), None);
    assert_eq!(read_shaped(&bad_bool, &bool::shape()), None);
    let mood = frame_of_uint(9);
    assert_eq!(read_shaped(&mood, &Mood::shape()), None);
    assert_eq!(read_frame::<Mood>(&mood), None);
    let atom = frame_of_uint(u64::MAX);
    assert_eq!(read_shaped(&atom, &Atom::shape()), None);
    assert_eq!(read_frame::<Atom>(&atom), None);
    let huge = frame_of_uint(u64::from(u32::MAX) + 1);
    assert_eq!(read_shaped(&huge, &Atom::shape()), None);
    let no_table = frame_of_uint(1);
    assert_eq!(read_shaped(&no_table, &String::shape()), None);
    assert_eq!(read_shaped(&no_table, &Vec::<u8>::shape()), None);
    assert_eq!(read_shaped(&no_table, &Option::<u8>::shape()), None);
    assert_eq!(read_shaped(&no_table, &Vec::<Entry>::shape()), None);
    assert_eq!(
        read_shaped(&frame_of_uint(u64::MAX), &Vec::<u16>::shape()),
        None
    );
    assert_eq!(read_shaped(&frame_of_str("a b"), &Id::<Doc>::shape()), None);
    assert_eq!(read_shaped(&frame_of_str("Bad"), &Ref::shape()), None);
}
