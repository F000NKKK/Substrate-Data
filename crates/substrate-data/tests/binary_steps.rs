//! The step-by-step decoder: each read answers `None` for the wrong thing.

use substrate_data::binary::{Decoder, Writer};
use substrate_data::{Schema, Stream};

#[test]
fn the_step_by_step_decoder_reads_scalars_and_answers_none_at_the_end() {
    let mut w = Writer::new();
    w.list(9);
    w.null()
        .bool(true)
        .bool(false)
        .int(-5)
        .float(1.5)
        .float32(2.5)
        .int(7);
    let bytes = w.finish();
    let mut d = Decoder::open(&bytes).unwrap();
    assert_eq!(d.list(), Some(9));
    assert!(d.null());
    assert!(!d.null());
    assert_eq!(d.bool(), Some(true));
    assert_eq!(d.bool(), Some(false));
    assert_eq!(d.int(), Some(-5));
    assert_eq!(d.float(), Some(1.5));
    assert_eq!(d.float(), Some(2.5));
    assert_eq!(d.float(), Some(7.0));
    assert_eq!((d.bool(), d.int(), d.float()), (None, None, None));
}

#[test]
fn the_step_by_step_decoder_reads_strings_bytes_and_packed_lists() {
    let mut w = Writer::new();
    w.list(5);
    w.int(7).str("s").bytes(&[1, 2]).ints([1i64].into_iter());
    w.floats(&[0.5]);
    let bytes = w.finish();
    let mut d = Decoder::open(&bytes).unwrap();
    assert_eq!(d.list(), Some(5));
    assert_eq!(d.int(), Some(7));
    assert_eq!(d.int(), None);
    assert_eq!(d.str(), Some("s"));
    assert_eq!(d.str(), None);
    assert_eq!(d.bytes(), Some(&[1u8, 2][..]));
    assert_eq!(d.ints(), Some(1));
    assert_eq!(d.packed_int(), Some(1));
    assert_eq!(d.floats(), Some(vec![0.5]));
    d.leave();
    assert!(d.at_end());
}

#[test]
fn packed_float_lists_read_step_by_step_in_both_widths() {
    let mut w = Writer::new();
    w.floats(&[0.1, 0.2])
        .floats(&[0.5])
        .floats32(&[1.5, 2.5])
        .floats32(&[3.5]);
    w.float(f64::NAN).bool(true);
    let bytes = w.finish();
    let mut d = Decoder::open(&bytes).unwrap();
    assert_eq!(d.floats32(), None);
    assert_eq!(d.floats(), Some(vec![0.1, 0.2]));
    assert_eq!(d.floats32(), Some(vec![0.5]));
    assert_eq!(d.floats(), Some(vec![1.5, 2.5]));
    assert_eq!(d.floats32(), Some(vec![3.5]));
    assert_eq!(d.floats(), None);
    assert!(d.float().is_some_and(f64::is_nan));
    assert_eq!(d.float(), None);
    assert_eq!(d.skip(), Some(()));
    assert!(d.at_end());
    assert_eq!(d.skip(), None);
}

#[test]
fn maps_and_streams_step_by_step() {
    let mut w = Writer::new();
    w.map(2).key("a").int(1).key("b").str("x");
    let s: Stream<u8> = [1u8, 2].into_iter().collect();
    s.encode(&mut w);
    let bytes = w.finish();
    let mut d = Decoder::open(&bytes).unwrap();
    assert_eq!(d.list(), None);
    assert_eq!(d.map(), Some(2));
    assert_eq!(d.key(), Some("a"));
    assert_eq!(d.int(), Some(1));
    assert_eq!(d.key(), Some("b"));
    assert_eq!(d.skip(), Some(()));
    d.leave();
    assert!(d.stream().is_some());
    assert!(d.at_end());
    d.leave();
}
