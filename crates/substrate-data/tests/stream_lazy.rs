//! Decoding a `Stream` checks its framing only; an item is read when it is
//! asked for, and a bad one is an error naming its index and byte.

#[path = "support/stream_doc.rs"]
mod stream_doc;

use stream_doc::{Narrow, Words, forged};
use substrate_data::binary::tag;
use substrate_data::{ItemError, Schema, Stream, Value, from_text, read_bytes, to_bytes, to_text};

#[derive(Schema, Debug, PartialEq)]
struct Wide {
    items: Stream<u32>,
}

/// Four numbers, the third too big for a `u8`: it reads as a `Narrow` to its
/// framing and fails at index 2, byte 2 (the first two take a byte each).
fn middle_bad() -> Vec<u8> {
    let wide = Wide {
        items: Stream::from(vec![1, 2, 300, 4]),
    };
    to_bytes(&wide)
}

#[test]
fn a_bad_item_in_the_middle_is_found_only_when_iterating() {
    let narrow = read_bytes::<Narrow>(&middle_bad()).unwrap();
    assert_eq!(narrow.items.len(), 4);
    let mut items = narrow.items.iter();
    assert_eq!(items.next(), Some(Ok(1)));
    assert_eq!(items.next(), Some(Ok(2)));
    assert_eq!(
        items.next(),
        Some(Err(ItemError {
            index: 2,
            offset: 2
        }))
    );
    // The items after it cannot be told apart: the iterator is over.
    assert_eq!(items.next(), None);
    assert_eq!(items.next(), None);
    assert_eq!(items.size_hint(), (0, Some(0)));
}

#[test]
fn collecting_stops_at_the_first_bad_item() {
    let narrow = read_bytes::<Narrow>(&middle_bad()).unwrap();
    let all: Result<Vec<u8>, _> = narrow.items.iter().collect();
    let error = all.unwrap_err();
    assert_eq!((error.index, error.offset), (2, 2));
    assert_eq!(narrow.items.to_vec(), Err(error));
    assert_eq!(narrow.items.validate(), Err(error));
    assert_eq!(error.to_string(), "stream item 2 at byte 2 cannot be read");
    let good = Stream::from(vec![1u8, 2, 3]);
    assert_eq!(good.validate(), Ok(()));
    assert_eq!(good.to_vec(), Ok(vec![1, 2, 3]));
}

#[test]
fn into_iterator_over_a_reference_is_iter() {
    let s = Stream::from(vec![7u16, 8]);
    let mut seen = Vec::new();
    for item in &s {
        seen.push(item.unwrap());
    }
    assert_eq!(seen, [7, 8]);
}

#[test]
fn the_offset_counts_bytes_of_items_before_it() {
    // Three strings, the last one cut short; the first two take `before` bytes.
    let words: Stream<String> = ["ab", "cde", "fghij"]
        .map(String::from)
        .into_iter()
        .collect();
    let before = ["ab", "cde"]
        .map(String::from)
        .into_iter()
        .collect::<Stream<String>>()
        .bytes_len();
    let doc = to_bytes(&Words {
        items: words.clone(),
    });
    let items = &doc[doc.len() - words.bytes_len()..];
    let cut = forged(3, &items[..items.len() - 2]);
    let read = read_bytes::<Words>(&cut).unwrap();
    let results: Vec<_> = read.items.iter().collect();
    assert_eq!(results.len(), 3);
    assert_eq!(results[0], Ok("ab".to_owned()));
    assert_eq!(results[1], Ok("cde".to_owned()));
    assert_eq!(
        results[2],
        Err(ItemError {
            index: 2,
            offset: before
        })
    );
}

#[test]
fn bytes_after_the_last_item_are_an_error_at_the_count() {
    let mut items = vec![145u8, 146];
    items.push(0xff);
    let narrow = read_bytes::<Narrow>(&forged(2, &items)).unwrap();
    let results: Vec<_> = narrow.items.iter().collect();
    assert_eq!(
        results,
        [
            Ok(1),
            Ok(2),
            Err(ItemError {
                index: 2,
                offset: 2
            })
        ]
    );
    assert_eq!(narrow.items.len(), 2);
    // Exactly its items: no error.
    let exact = read_bytes::<Narrow>(&forged(2, &items[..2])).unwrap();
    assert_eq!(exact.items.to_vec(), Ok(vec![1, 2]));
}

#[test]
fn an_item_cut_short_at_the_end_is_an_error_at_that_item() {
    // Three counted, the third a string tag and nothing after it.
    let narrow = read_bytes::<Narrow>(&forged(3, &[145, 146, tag::STR])).unwrap();
    let results: Vec<_> = narrow.items.iter().collect();
    assert_eq!(
        results,
        [
            Ok(1),
            Ok(2),
            Err(ItemError {
                index: 2,
                offset: 2
            })
        ]
    );
    // More counted than the bytes could hold even at one byte each: refused
    // when it is decoded.
    assert!(read_bytes::<Narrow>(&forged(3, &[145, 146])).is_err());
}

#[test]
fn equality_is_the_count_and_the_bytes() {
    let narrow = read_bytes::<Narrow>(&middle_bad()).unwrap();
    // A stream with an unreadable item equals a copy of itself, and nothing else.
    assert_eq!(narrow.items.clone(), narrow.items);
    assert_eq!(
        narrow.items,
        read_bytes::<Narrow>(&middle_bad()).unwrap().items
    );
    assert_ne!(narrow.items, Stream::from(vec![1u8, 2]));
    // The same items written in another valid spelling (an int with its own
    // tag, as version 1 wrote it) are not equal: bytes are compared, not items.
    let plain = Stream::from(vec![1u8]);
    let spelled = read_bytes::<Narrow>(&forged(1, &[tag::INT, 2]))
        .unwrap()
        .items;
    assert_eq!(spelled.to_vec(), plain.to_vec());
    assert_ne!(spelled, plain);
    assert_eq!(format!("{plain:?}"), "Stream(1 items, 1 bytes)");
}

#[test]
fn the_value_and_text_forms_show_the_items_that_read() {
    let narrow = read_bytes::<Narrow>(&middle_bad()).unwrap();
    // A `Value` has no place for an error: it holds the items before the bad one.
    let value = narrow.items.write();
    assert_eq!(value, Value::List(vec![Value::Int(1), Value::Int(2)]));
    let text = to_text(&narrow);
    let from_text = from_text::<Narrow>(&text).unwrap().value;
    assert_eq!(from_text.items.to_vec(), Ok(vec![1, 2]));
    // Written again in binary it is what was read: nothing was lost on the way.
    assert_eq!(to_bytes(&narrow), middle_bad());
}

#[test]
fn a_frame_packs_the_items_that_read() {
    use substrate_data::frame::{read_frame, to_frame};
    let narrow = read_bytes::<Narrow>(&middle_bad()).unwrap();
    let back = read_frame::<Narrow>(&to_frame(&narrow)).unwrap();
    assert_eq!(back.items.to_vec(), Ok(vec![1, 2]));
    let good = Narrow {
        items: Stream::from(vec![9, 8]),
    };
    assert_eq!(read_frame::<Narrow>(&to_frame(&good)), Some(good));
}

#[test]
fn collecting_and_converting_build_the_same_stream() {
    let from_vec = Stream::from(vec![1u16, 2, 3]);
    let from_iter: Stream<u16> = (1..=3).collect();
    let mut w = Stream::<u16>::writer();
    for i in 1..=3 {
        w.push(&i);
    }
    assert_eq!(from_vec, from_iter);
    assert_eq!(from_vec, w.finish());
    assert_eq!(from_vec.iter().size_hint(), (0, Some(4)));
}
