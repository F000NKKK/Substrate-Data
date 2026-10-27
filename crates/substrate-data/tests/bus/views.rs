//! Views: a frame read without copying.

use substrate_data::Schema;
use substrate_data::frame::{read_frame, read_view, to_frame};

#[derive(Schema, Debug, PartialEq)]
#[data(view)]
struct Hit {
    id: String,
    count: u32,
    weights: Vec<f64>,
    entries: Vec<i16>,
    tags: Vec<String>,
    inner: Option<Inner>,
}

#[derive(Schema, Debug, PartialEq)]
#[data(view)]
struct Inner {
    name: String,
}

#[test]
fn a_view_borrows_and_reads_in_place() {
    let hit = Hit {
        id: "docs/intro".into(),
        count: 7,
        weights: vec![0.5, 0.25],
        entries: vec![-3, 0, 900],
        tags: vec!["a".into(), "b".into()],
        inner: Some(Inner { name: "in".into() }),
    };
    let frame = to_frame(&hit);
    let v = read_view::<Hit>(&frame).unwrap();
    assert_eq!(v.id, "docs/intro");
    // The string points into the frame: nothing copied.
    assert!(frame.as_ptr_range().contains(&v.id.as_ptr()));
    assert_eq!(v.count, 7);
    assert_eq!(v.weights.iter().collect::<Vec<f64>>(), [0.5, 0.25]);
    assert_eq!(v.entries.iter().collect::<Vec<i16>>(), [-3, 0, 900]);
    assert_eq!(v.entries.get(2), Some(900));
    assert_eq!(v.entries.get(3), None);
    // A list read lazily, item by item.
    assert_eq!(v.tags.iter().collect::<Vec<&str>>(), ["a", "b"]);
    assert_eq!(
        (v.tags.len(), v.tags.get(1), v.tags.get(2)),
        (2, Some("b"), None)
    );
    assert_eq!(v.inner.map(|i| i.name), Some("in"));
}

#[derive(Schema, Debug, PartialEq)]
#[data(view)]
struct Shelf {
    rows: Vec<Vec<Inner>>,
    after: u32,
}

#[test]
fn lists_are_stepped_over_and_read_lazily() {
    let shelf = Shelf {
        rows: vec![
            vec![Inner { name: "a".into() }, Inner { name: "b".into() }],
            vec![],
            vec![Inner { name: "c".into() }],
        ],
        after: 42,
    };
    let frame = to_frame(&shelf);
    let v = read_view::<Shelf>(&frame).unwrap();
    // The field after the lists is reached without reading them.
    assert_eq!(v.after, 42);
    assert_eq!(v.rows.len(), 3);
    let names: Vec<Vec<&str>> = v
        .rows
        .iter()
        .map(|row| row.iter().map(|i| i.name).collect())
        .collect();
    assert_eq!(names, [vec!["a", "b"], vec![], vec!["c"]]);
    assert_eq!(
        v.rows.get(2).and_then(|r| r.get(0)).map(|i| i.name),
        Some("c")
    );
    assert!(v.rows.get(3).is_none());
    assert_eq!(read_frame::<Shelf>(&frame), Some(shelf));
}
