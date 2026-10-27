//! Streams: lists kept in the binary form, written item by item and read
//! back one at a time, in every form.

use substrate_data::frame::{read_frame, to_frame};
use substrate_data::{Schema, Stream, Value, from_text, read_bytes, read_value, to_bytes, to_text};

#[derive(Schema, Debug, Clone, PartialEq)]
struct Row {
    label: String,
    cells: Vec<u16>,
}

#[derive(Schema, Debug, Clone, PartialEq)]
struct Table {
    name: String,
    rows: Stream<Row>,
}

fn table() -> Table {
    Table {
        name: "q3".into(),
        rows: (0..5u16)
            .map(|i| Row {
                label: format!("row {i}"),
                cells: vec![i, i + 1],
            })
            .collect(),
    }
}

#[test]
fn a_stream_is_written_and_read_item_by_item() {
    let mut w = Stream::<u32>::writer();
    for i in 0..4u32 {
        w.push(&(i * 1000));
    }
    let s = w.finish();
    assert_eq!((s.len(), s.is_empty()), (4, false));
    assert_eq!(s.to_vec(), [0, 1000, 2000, 3000]);
    assert_eq!(s.iter().nth(2), Some(2000));
    assert!(s.bytes_len() > 0);
    assert_eq!(
        format!("{s:?}"),
        format!("Stream(4 items, {} bytes)", s.bytes_len())
    );
    let empty = Stream::<u32>::default();
    assert!(empty.is_empty() && empty.bytes_len() == 0);
    assert_eq!(empty, Stream::from(Vec::new()));
    assert_eq!(s.clone(), s);
    assert_ne!(s, Stream::from(vec![0, 1000, 2000, 3001]));
    assert_ne!(s, empty);
}

#[test]
fn a_stream_round_trips_in_every_form() {
    let t = table();
    assert_eq!(read_value::<Table>(&t.write()).unwrap(), t);
    let text = to_text(&t);
    assert_eq!(from_text::<Table>(&text).unwrap().value, t);
    assert_eq!(read_bytes::<Table>(&to_bytes(&t)).unwrap(), t);
    assert_eq!(read_frame::<Table>(&to_frame(&t)), Some(t.clone()));
    assert_eq!(t.rows.write().as_list().map(<[Value]>::len), Some(5));
    let empty = Table {
        name: "e".into(),
        rows: Stream::default(),
    };
    assert_eq!(read_bytes::<Table>(&to_bytes(&empty)).unwrap(), empty);
}

#[test]
fn a_stream_with_a_wrong_item_names_its_place() {
    let mut v = table().write();
    let Value::Map(m) = &mut v else { panic!() };
    m.insert("rows", Value::List(vec![Value::Int(1)]));
    let item = read_value::<Table>(&v).unwrap_err();
    assert_eq!(item[0].path.to_string(), "rows[0]");
    let Value::Map(entries) = &mut v else {
        panic!()
    };
    entries.insert("rows", Value::Int(1));
    let whole = read_value::<Table>(&v).unwrap_err();
    assert!(whole[0].message.contains("expected a list"));
}

#[test]
fn a_cut_stream_is_an_error_not_a_panic() {
    let bytes = to_bytes(&table());
    for cut in 0..bytes.len() {
        assert!(read_bytes::<Table>(&bytes[..cut]).is_err(), "cut at {cut}");
    }
    let frame = to_frame(&table());
    for cut in 0..frame.len() {
        assert_eq!(
            read_frame::<Table>(&frame[..cut]),
            None,
            "frame cut at {cut}"
        );
    }
}
