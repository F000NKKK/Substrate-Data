//! Frames: big records and many strings, frames read by their shape alone,
//! the packer and unpacker used directly, and frames that are cut or lie.

use std::sync::Arc;

use substrate_data::frame::{Packer, Unpacker, read_frame, read_shaped, to_frame};
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
        anything: Ref::parse("file", "a/b").unwrap(),
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

#[test]
fn a_frame_trailer_longer_than_64_bits_is_no_frame() {
    let mut frame = vec![0x02];
    frame.extend_from_slice(&[0x80; 9]);
    assert!(Unpacker::open(&frame).is_none());
    let mut fits = vec![0x01];
    fits.extend_from_slice(&[0x80; 9]);
    assert!(
        Unpacker::open(&fits).is_none(),
        "a table of 2^63 bytes isn't there"
    );
    assert!(Unpacker::open(&[0x00]).is_some());
    assert!(Unpacker::open(&[]).is_none());
    assert!(Unpacker::open(&[0x80]).is_none());
}

#[test]
fn big_records_and_many_strings_round_trip() {
    #[derive(Schema, Debug, Clone, PartialEq)]
    struct Big {
        items: Vec<String>,
        shared: Vec<Arc<str>>,
        long: String,
        inner: Vec<Entry>,
    }
    let x = Big {
        items: (0..60).map(|i| format!("string number {i}")).collect(),
        shared: (0..40).map(|i| Arc::from(format!("s{}", i % 20))).collect(),
        long: "x".repeat(300),
        inner: (0..30)
            .map(|i| Entry {
                title: format!("entry {}", i % 25),
                weight: i as f32,
            })
            .collect(),
    };
    let frame = to_frame(&x);
    assert_eq!(read_frame::<Big>(&frame), Some(x.clone()));
    let back = read_frame::<Big>(&frame).unwrap();
    assert!(Arc::ptr_eq(&back.shared[0], &back.shared[20]));
    assert_eq!(
        substrate_data::read_bytes::<Big>(&substrate_data::to_bytes(&x)).unwrap(),
        x
    );
    for cut in [0, 1, 10, frame.len() / 2, frame.len() - 1] {
        assert_eq!(read_frame::<Big>(&frame[..cut]), None, "cut at {cut}");
    }
}

#[test]
fn the_packer_and_unpacker_are_used_piece_by_piece() {
    let mut p = Packer::new();
    p.put_u(7);
    p.put_i(-7);
    p.put_bool(true);
    p.put_f64(1.5);
    p.put_f32(2.5);
    p.put_str("same");
    p.put_str("same");
    p.put_bytes(&[1, 2, 3]);
    p.put_f64s(&[0.1, 0.2]);
    p.put_f64s(&[0.5]);
    p.put_f32s(&[1.0, 2.0]);
    p.put_uints(3, [1u64, 500, 70_000].into_iter());
    p.put_tagged(&vec![String::from("t")]);
    let at = p.begin_list(2);
    p.put_u(1);
    p.put_u(2);
    p.end(at);
    let frame = p.finish();
    let mut u = Unpacker::open(&frame).unwrap();
    assert_eq!(u.uint(), Some(7));
    assert_eq!(u.int(), Some(-7));
    assert_eq!(u.bool(), Some(true));
    assert_eq!(u.f64(), Some(1.5));
    assert_eq!(u.f32(), Some(2.5));
    assert_eq!(u.str(), Some("same"));
    assert_eq!(u.shared_str().as_deref(), Some("same"));
    assert_eq!(u.bytes(), Some(&[1u8, 2, 3][..]));
    assert_eq!(u.f64s(), Some(vec![0.1, 0.2]));
    assert_eq!(u.f64s(), Some(vec![0.5]));
    assert_eq!(u.f32s(), Some(vec![1.0, 2.0]));
    assert_eq!(u.uints(Some), Some(vec![1u64, 500, 70_000]));
    assert_eq!(u.tagged::<Vec<String>>(), Some(vec!["t".to_owned()]));
    let (n, end) = u.list().unwrap();
    assert_eq!(n, 2);
    let here = u.position();
    let mut fork = u.fork(here);
    assert_eq!(fork.uint(), Some(1));
    assert_eq!(u.skip_to(end), Some(()));
    assert!(u.at_end());
    assert_eq!(u.skip_to(0), None);
    assert_eq!(u.skip_to(usize::MAX), None);
    assert_eq!(u.uint(), None);
}

#[test]
fn a_packer_is_reused_without_leaking_between_frames() {
    let mut p = Packer::new();
    let a = p.with_frame(&"first".to_owned(), <[u8]>::to_vec);
    let b = p.with_frame(&"second".to_owned(), <[u8]>::to_vec);
    let a_again = p.with_frame(&"first".to_owned(), <[u8]>::to_vec);
    assert_eq!(a, a_again);
    assert_eq!(read_frame::<String>(&b).as_deref(), Some("second"));
    assert_eq!(a, to_frame(&"first".to_owned()));
}

#[test]
fn counts_beyond_what_is_left_are_refused_before_allocating() {
    let mut p = Packer::new();
    p.put_u(u64::MAX / 2);
    let frame = p.finish();
    let mut u = Unpacker::open(&frame).unwrap();
    assert_eq!(u.count(), None);
    let mut u = Unpacker::open(&frame).unwrap();
    assert_eq!(u.bytes(), None);
    let mut u = Unpacker::open(&frame).unwrap();
    assert_eq!(u.f64s(), None);
    let mut u = Unpacker::open(&frame).unwrap();
    assert_eq!(u.packed_ints(), None);
    let mut u = Unpacker::open(&frame).unwrap();
    assert_eq!(u.list(), None);
    let mut u = Unpacker::open(&frame).unwrap();
    assert_eq!(u.begin(), None);
}

#[test]
fn nesting_deeper_than_the_limit_is_no_frame() {
    let frame = to_frame(&vec![vec![vec![1u8]]]);
    assert_eq!(
        read_frame::<Vec<Vec<Vec<u8>>>>(&frame).map(|v| v.len()),
        Some(1)
    );
    let mut p = Packer::new();
    let mut opened = Vec::new();
    for _ in 0..300 {
        opened.push(p.begin());
    }
    for at in opened.into_iter().rev() {
        p.end(at);
    }
    let frame = p.finish();
    let mut un = Unpacker::open(&frame).unwrap();
    let mut depth = 0;
    while un.begin().is_some() {
        depth += 1;
    }
    assert!(depth < 300, "stopped at {depth}");
}
