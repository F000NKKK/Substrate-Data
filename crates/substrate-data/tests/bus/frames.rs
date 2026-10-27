//! Frames: packed by position.

use substrate_data::Schema;
use substrate_data::frame::{read_frame, to_frame};

use super::{Compact, Entry, Folder, Message, Mode};

/// 64-bit unsigned numbers, every bit used (a packed timestamp and id).
#[derive(Schema, Debug, Clone, PartialEq)]
struct Wide {
    key: u64,
    size: usize,
    parent: Option<u64>,
    keys: Vec<u64>,
}

#[test]
fn unsigned_numbers_keep_all_64_bits() {
    for top in [0, 1u64 << 62, 1 << 63, u64::MAX] {
        let w = Wide {
            key: top,
            size: usize::try_from(top).unwrap_or(usize::MAX),
            parent: Some(top ^ 1),
            keys: vec![top, 0, top >> 1],
        };
        assert_eq!(
            read_frame::<Wide>(&to_frame(&w)),
            Some(w.clone()),
            "frame {top:#x}"
        );
        let bytes = substrate_data::to_bytes(&w);
        assert_eq!(
            substrate_data::read_bytes::<Wide>(&bytes).ok(),
            Some(w.clone()),
            "binary {top:#x}"
        );
        assert_eq!(
            substrate_data::from_value::<Wide>(&w.write())
                .ok()
                .map(|r| r.value),
            Some(w),
            "value {top:#x}"
        );
    }
}

#[test]
fn every_kind_of_field_round_trips() {
    let m = Message {
        name: "report".into(),
        count: 3,
        ratio: 0.5,
        tags: vec!["a".into(), "b".into(), "a".into()],
        mode: Mode::Archived,
        note: Some("n".into()),
        extra: -7,
        bytes: vec![1, 2, 3],
    };
    let frame = to_frame(&m);
    assert_eq!(read_frame::<Message>(&frame), Some(m.clone()));
    // Smaller than the self-describing form: no keys, no tags.
    assert!(
        frame.len() < substrate_data::to_bytes(&m).len(),
        "{} vs {}",
        frame.len(),
        substrate_data::to_bytes(&m).len()
    );
    let c = Compact {
        name: "a".into(),
        shared: "a".into(),
        ids: vec![1, 2].into_boxed_slice(),
        weights: vec![0.5, 1.25],
        scale: 2.5,
    };
    assert_eq!(read_frame::<Compact>(&to_frame(&c)), Some(c));
    // A stream goes as its binary form inside the frame.
    let entries: substrate_data::Stream<Entry> = (0..10)
        .map(|i| Entry {
            id: "docs/intro".into(),
            count: i,
        })
        .collect();
    let folder = Folder {
        name: "pit".into(),
        owners: vec!["docs/intro".into()],
        entries,
        after: "x".into(),
    };
    assert_eq!(read_frame::<Folder>(&to_frame(&folder)), Some(folder));
}

/// A frame packed in a reused packer's buffers is the same frame.
#[test]
fn reused_buffers_make_the_same_frames() {
    let a = vec!["x".to_owned(), "y".to_owned(), "x".to_owned()];
    let b = vec![1u32, 2, 3];
    let mut p = substrate_data::frame::Packer::new();
    for _ in 0..3 {
        p.with_frame(&a, |fa| assert_eq!(fa, to_frame(&a).as_slice()));
        p.with_frame(&b, |fb| assert_eq!(fb, to_frame(&b).as_slice()));
    }
}

#[test]
fn repeated_strings_are_stored_once() {
    let tags: Vec<String> = (0..100).map(|_| "docs/intro".to_owned()).collect();
    let frame = to_frame(&tags);
    assert_eq!(frame.windows(10).filter(|w| w == b"docs/intro").count(), 1);
    assert_eq!(read_frame::<Vec<String>>(&frame), Some(tags));
}

#[derive(Schema, Debug, PartialEq)]
struct Old {
    a: u32,
    b: String,
}

#[derive(Schema, Debug, PartialEq)]
struct New {
    a: u32,
    b: String,
    #[data(default)]
    c: Vec<String>,
    d: Option<f64>,
}

#[derive(Schema, Debug, PartialEq)]
struct Holds<T: Schema> {
    inner: T,
    after: String,
}

#[test]
fn fields_are_appended_and_read_both_ways() {
    // A newer writer's fields (with strings of their own) are skipped by an older reader…
    let new = Holds {
        inner: New {
            a: 1,
            b: "x".into(),
            c: vec!["only new".into()],
            d: Some(2.0),
        },
        after: "y".into(),
    };
    let old: Holds<Old> = read_frame(&to_frame(&new)).unwrap();
    assert_eq!(
        old,
        Holds {
            inner: Old {
                a: 1,
                b: "x".into()
            },
            after: "y".into()
        }
    );
    // …and an older writer's record takes a newer reader's defaults.
    let back: Holds<New> = read_frame(&to_frame(&old)).unwrap();
    assert_eq!(
        back.inner,
        New {
            a: 1,
            b: "x".into(),
            c: Vec::new(),
            d: None
        }
    );
}

#[test]
fn numbers_pack_tight() {
    // Palette indices 0..=2: two bits each.
    let entries: Vec<u16> = (0..32768u16).map(|i| i % 3).collect();
    let frame = to_frame(&entries);
    assert!(frame.len() < 32768 / 4 + 16, "{} bytes", frame.len());
    assert_eq!(read_frame::<Vec<u16>>(&frame), Some(entries));
    let signed = vec![0i32, -1, 1, i32::MIN, i32::MAX];
    assert_eq!(read_frame::<Vec<i32>>(&to_frame(&signed)), Some(signed));
    let wide = vec![u64::MAX, 0, 1 << 63];
    assert_eq!(read_frame::<Vec<u64>>(&to_frame(&wide)), Some(wide));
    let zeros = vec![0u8; 100];
    assert_eq!(read_frame::<Vec<u8>>(&to_frame(&zeros)), Some(zeros));
    // Floats that are `f32`s take four bytes; one that isn't keeps them all eight.
    let halves: Vec<f64> = (0..1000).map(|i| f64::from(i) * 0.5).collect();
    assert!(to_frame(&halves).len() < 1000 * 4 + 16);
    assert_eq!(read_frame::<Vec<f64>>(&to_frame(&halves)), Some(halves));
    let exact = vec![0.1f64, 2.0];
    assert_eq!(read_frame::<Vec<f64>>(&to_frame(&exact)), Some(exact));
}

/// What a tool read by the shape alone is the value itself.
#[test]
fn a_frame_reads_by_its_shape_alone() {
    use substrate_data::frame::read_shaped;
    let m = Message {
        name: "report".into(),
        count: 3,
        ratio: 0.5,
        tags: vec!["a".into(), "b".into()],
        mode: Mode::Archived,
        note: None,
        extra: -7,
        bytes: vec![1, 2, 3],
    };
    let value = read_shaped(&to_frame(&m), &Message::shape()).unwrap();
    assert_eq!(substrate_data::read_value::<Message>(&value).unwrap(), m);
    let entries: substrate_data::Stream<Entry> = (0..3)
        .map(|i| Entry {
            id: "docs/intro".into(),
            count: i,
        })
        .collect();
    let folder = Folder {
        name: "pit".into(),
        owners: vec!["docs/intro".into()],
        entries,
        after: "x".into(),
    };
    let seen = read_shaped(&to_frame(&folder), &Folder::shape()).unwrap();
    assert_eq!(substrate_data::read_value::<Folder>(&seen).unwrap(), folder);
    let c = Compact {
        name: "a".into(),
        shared: "a".into(),
        ids: vec![1, 2].into_boxed_slice(),
        weights: vec![0.5, 1.25],
        scale: 2.5,
    };
    let seen_compact = read_shaped(&to_frame(&c), &Compact::shape()).unwrap();
    assert_eq!(
        substrate_data::read_value::<Compact>(&seen_compact).unwrap(),
        c
    );
}

#[test]
fn malformed_frames_are_none() {
    let frame = to_frame(&Holds {
        inner: New {
            a: 1,
            b: "x".into(),
            c: vec!["c".into()],
            d: None,
        },
        after: "y".into(),
    });
    for len in 0..frame.len() {
        assert!(
            read_frame::<Holds<New>>(&frame[..len]).is_none(),
            "cut at {len}"
        );
    }
    // A count bigger than the bytes left is refused before anything is allocated.
    let mut p = substrate_data::frame::Packer::new();
    p.put_u(u64::MAX >> 1);
    assert!(read_frame::<Vec<u32>>(&p.finish()).is_none());
}
