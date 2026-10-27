//! Frames: big records and many strings, the packer and unpacker used
//! directly, and frames that lie about their length.

use std::sync::Arc;

use substrate_data::Schema;
use substrate_data::frame::{Packer, Unpacker, read_frame, to_frame};

#[derive(Schema, Debug, Clone, PartialEq)]
struct Entry {
    title: String,
    weight: f32,
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
        inner: (0..30u8)
            .map(|i| Entry {
                title: format!("entry {}", i % 25),
                weight: f32::from(i),
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
    assert!(u.at_end());
}

#[test]
fn lists_and_nested_values_are_packed_and_read_piece_by_piece() {
    let mut p = Packer::new();
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

/// Reading `read` from a frame holding one huge count is refused.
fn assert_refused(refused: impl FnOnce(&mut Unpacker<'_>) -> bool) {
    let mut p = Packer::new();
    p.put_u(u64::MAX / 2);
    let frame = p.finish();
    let mut u = Unpacker::open(&frame).expect("a frame");
    assert!(refused(&mut u));
}

#[test]
fn counts_beyond_what_is_left_are_refused_before_allocating() {
    assert_refused(|u| u.count().is_none());
    assert_refused(|u| u.bytes().is_none());
    assert_refused(|u| u.f64s().is_none());
    assert_refused(|u| u.packed_ints().is_none());
    assert_refused(|u| u.list().is_none());
    assert_refused(|u| u.begin().is_none());
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
    let deep = p.finish();
    let mut un = Unpacker::open(&deep).unwrap();
    let mut depth = 0;
    while un.begin().is_some() {
        depth += 1;
    }
    assert!(depth < 300, "stopped at {depth}");
}
