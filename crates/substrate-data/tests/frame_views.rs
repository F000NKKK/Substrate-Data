//! Views over frames: every kind of field read in place, lists of every
//! item type, and frames that are cut or wrong read as `None`.

use substrate_data::frame::{read_view, to_frame};
use substrate_data::{Atom, Schema};

#[derive(Schema, Debug, PartialEq)]
#[data(view)]
struct Everything {
    flag: bool,
    flags: Vec<bool>,
    boxed: Box<str>,
    names: Vec<Box<str>>,
    atom: Atom,
    atoms: Vec<Atom>,
    maybe: Option<u8>,
    maybes: Vec<Option<u8>>,
    nested: Box<u16>,
    bytes: Vec<u8>,
    words: Vec<u16>,
    longs: Vec<u32>,
    wide: Vec<u64>,
    signed8: Vec<i8>,
    signed16: Vec<i16>,
    signed32: Vec<i32>,
    signed64: Vec<i64>,
    sizes: Vec<usize>,
    narrow: Vec<f32>,
    exact: Vec<f64>,
    fine: Vec<f64>,
    single: f32,
    big: i64,
}

fn sample() -> Everything {
    Everything {
        flag: true,
        flags: vec![true, false, true],
        boxed: "box".into(),
        names: vec!["x".into(), "yy".into()],
        atom: Atom::new("view_atom"),
        atoms: vec![Atom::new("view_atom"), Atom::new("view_other")],
        maybe: Some(4),
        maybes: vec![Some(1), None, Some(3)],
        nested: Box::new(500),
        bytes: vec![1, 2, 255],
        words: vec![1, 65_535, 300],
        longs: vec![0, 4_000_000_000],
        wide: vec![u64::MAX, 1],
        signed8: vec![-128, 127, 0],
        signed16: vec![-30_000, 30_000],
        signed32: vec![-2_000_000_000, 5],
        signed64: vec![i64::MIN, i64::MAX, 0],
        sizes: vec![3, 1 << 40],
        narrow: vec![0.5, 1.5, -2.25],
        exact: vec![0.5, 8.0],
        fine: vec![0.1, 1e300],
        single: 2.5,
        big: -9,
    }
}

#[test]
fn every_kind_of_field_is_read_in_place() {
    let x = sample();
    let frame = to_frame(&x);
    let v = read_view::<Everything>(&frame).unwrap();
    assert!(v.flag && v.flags.len() == 3 && !v.flags.is_empty());
    assert_eq!(v.flags.iter().collect::<Vec<_>>(), [true, false, true]);
    assert_eq!(v.boxed, "box");
    assert_eq!(v.names.iter().collect::<Vec<_>>(), ["x", "yy"]);
    assert_eq!(v.atom, x.atom);
    assert_eq!(v.atoms.iter().collect::<Vec<_>>(), x.atoms);
    assert_eq!(v.maybe, Some(4));
    assert_eq!(
        v.maybes.iter().collect::<Vec<_>>(),
        [Some(1), None, Some(3)]
    );
    assert_eq!(v.nested, 500);
}

#[test]
fn packed_integer_lists_are_read_in_place() {
    let x = sample();
    let frame = to_frame(&x);
    let v = read_view::<Everything>(&frame).unwrap();
    assert_eq!(v.bytes.iter().collect::<Vec<_>>(), x.bytes);
    assert_eq!(v.words.iter().collect::<Vec<_>>(), x.words);
    assert_eq!(v.longs.iter().collect::<Vec<_>>(), x.longs);
    assert_eq!(v.wide.iter().collect::<Vec<_>>(), x.wide);
    assert_eq!(v.signed8.iter().collect::<Vec<_>>(), x.signed8);
    assert_eq!(v.signed16.iter().collect::<Vec<_>>(), x.signed16);
    assert_eq!(v.signed32.iter().collect::<Vec<_>>(), x.signed32);
    assert_eq!(v.signed64.iter().collect::<Vec<_>>(), x.signed64);
    assert_eq!(v.sizes.iter().collect::<Vec<_>>(), x.sizes);
}

#[test]
fn packed_float_lists_are_read_in_place() {
    let x = sample();
    let frame = to_frame(&x);
    let v = read_view::<Everything>(&frame).unwrap();
    assert_eq!(v.narrow.iter().collect::<Vec<_>>(), x.narrow);
    assert_eq!(v.exact.iter().collect::<Vec<f64>>(), x.exact);
    assert_eq!(v.fine.iter().collect::<Vec<f64>>(), x.fine);
    assert_eq!((v.single, v.big), (2.5, -9));
}

#[test]
fn packed_lists_know_their_size_and_stop_at_the_end() {
    let frame = to_frame(&sample());
    let v = read_view::<Everything>(&frame).unwrap();
    assert_eq!((v.words.len(), v.words.is_empty()), (3, false));
    assert_eq!(v.words.get(1), Some(65_535));
    assert_eq!(v.words.get(3), None);
    assert_eq!((v.narrow.len(), v.narrow.is_empty()), (3, false));
    assert_eq!(v.narrow.get(2), Some(-2.25));
    assert_eq!(v.narrow.get(3), None);
    assert_eq!(v.fine.get(1), Some(1e300));
    assert_eq!(v.fine.get(2), None);
    let lists = v.flags.clone();
    assert_eq!(format!("{lists:?}"), "List(3 items)");
    assert_eq!((&lists).into_iter().count(), 3);
    assert_eq!(lists.get(2), Some(true));
    assert_eq!(lists.get(3), None);
}

#[derive(Schema, Debug, PartialEq)]
#[data(view)]
struct Rows {
    rows: Vec<Vec<u8>>,
    none: Vec<String>,
}

#[test]
fn empty_lists_view_as_empty() {
    let frame = to_frame(&Rows {
        rows: vec![vec![], vec![1]],
        none: vec![],
    });
    let v = read_view::<Rows>(&frame).unwrap();
    assert_eq!(v.rows.len(), 2);
    assert_eq!(v.rows.iter().map(|r| r.len()).collect::<Vec<_>>(), [0, 1]);
    assert!(v.none.is_empty());
}

#[test]
fn a_cut_or_wrong_frame_is_no_view() {
    let frame = to_frame(&sample());
    for cut in 0..frame.len() {
        assert!(
            read_view::<Everything>(&frame[..cut]).is_none(),
            "cut at {cut}"
        );
    }
    let mut longer = frame.clone();
    longer.push(0);
    assert!(read_view::<Everything>(&longer).is_none());
    let other = to_frame(&Rows {
        rows: vec![],
        none: vec![],
    });
    assert!(read_view::<Everything>(&other).is_none());
    assert!(read_view::<Everything>(&[]).is_none());
}
