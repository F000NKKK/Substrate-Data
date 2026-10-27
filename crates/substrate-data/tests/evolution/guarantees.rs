//! The promises the document makes, one test each, in words.

use substrate_data::diagnostic::{Code, Problem};
use substrate_data::frame::{read_frame, read_shaped, to_frame};
use substrate_data::{Schema, Stream, Value, from_text, read_bytes, to_bytes, to_text};

use crate::base::{Doc, Grown, In, base, grown};

fn problems(refusal: &[substrate_data::Diagnostic]) -> Vec<Problem> {
    refusal
        .iter()
        .filter_map(|d| match d.code {
            Some(Code::Data(p)) => Some(p),
            _ => None,
        })
        .collect()
}

#[test]
fn frames_skip_appended_fields_and_default_missing_ones() {
    let older = In {
        v: base(),
        after: 5,
    };
    let newer = In {
        v: grown(&["a"]),
        after: 5,
    };
    assert_eq!(read_frame::<In<Doc>>(&to_frame(&newer)), Some(older));
    let filled = read_frame::<In<Grown>>(&to_frame(&In {
        v: base(),
        after: 5,
    }));
    assert_eq!(
        filled,
        Some(In {
            v: grown(&[]),
            after: 5
        })
    );
}

#[test]
fn named_forms_refuse_a_field_the_reader_does_not_know() {
    let newer = grown(&["a"]);
    assert_eq!(
        problems(&read_bytes::<Doc>(&to_bytes(&newer)).unwrap_err()),
        [Problem::UnknownField]
    );
    let text = from_text::<Doc>(&to_text(&newer)).unwrap_err();
    assert_eq!(text.0[0].code, Some(Code::Data(Problem::UnknownField)));
}

#[test]
fn a_missing_field_is_filled_only_by_default_or_option() {
    #[derive(Schema, Debug, PartialEq)]
    struct Needs {
        name: String,
        n: u32,
        level: u32,
    }
    assert_eq!(read_frame::<Needs>(&to_frame(&base())), None);
    assert_eq!(
        problems(&read_bytes::<Needs>(&to_bytes(&base())).unwrap_err()),
        [Problem::MissingField]
    );
}

#[test]
fn a_reorder_is_silent_in_frames_and_harmless_in_named_forms() {
    #[derive(Schema, Debug, PartialEq)]
    struct Pair {
        a: String,
        b: String,
    }
    #[derive(Schema, Debug, PartialEq)]
    struct Flipped {
        b: String,
        a: String,
    }
    let pair = Pair {
        a: "first".into(),
        b: "second".into(),
    };
    let wrong = read_frame::<Flipped>(&to_frame(&pair)).unwrap();
    assert_eq!((wrong.b.as_str(), wrong.a.as_str()), ("first", "second"));
    let right = read_bytes::<Flipped>(&to_bytes(&pair)).unwrap();
    assert_eq!((right.b.as_str(), right.a.as_str()), ("second", "first"));
}

#[test]
fn a_new_unit_variant_is_refused_by_an_older_reader_in_every_form() {
    #[derive(Schema, Debug, PartialEq)]
    enum Old {
        A,
    }
    #[derive(Schema, Debug, PartialEq)]
    enum New {
        A,
        B,
    }
    assert_eq!(read_frame::<Old>(&to_frame(&New::B)), None);
    assert_eq!(
        problems(&read_bytes::<Old>(&to_bytes(&New::B)).unwrap_err()),
        [Problem::Invalid]
    );
    assert!(from_text::<Old>(&to_text(&New::B)).is_err());
    assert_eq!(read_frame::<New>(&to_frame(&Old::A)), Some(New::A));
}

#[test]
fn a_frame_read_by_shape_leaves_out_the_fields_it_does_not_have() {
    let frame = to_frame(&base());
    let shape = Grown::shape();
    let Value::Map(map) = read_shaped(&frame, &shape).unwrap() else {
        panic!("a record reads as a map");
    };
    assert_eq!(map.len(), 2, "tags is absent, not defaulted");
    assert!(map.get("tags").is_none());
}

#[test]
fn a_stream_checks_its_items_only_when_they_are_read() {
    #[derive(Schema, Debug, PartialEq)]
    struct Items<T: Schema> {
        items: Stream<T>,
    }
    let newer = Items {
        items: Stream::from(vec![grown(&["a"])]),
    };
    let older = read_bytes::<Items<Doc>>(&to_bytes(&newer)).unwrap();
    assert_eq!(older.items.len(), 1);
    assert!(
        older.items.to_vec().is_empty(),
        "iteration stops at the item it cannot read"
    );
    let oldest = Items {
        items: Stream::from(vec![base()]),
    };
    let regrown = read_bytes::<Items<Grown>>(&to_bytes(&oldest)).unwrap();
    assert_eq!(regrown.items.to_vec(), [grown(&[])]);
}

#[derive(Schema, Debug, PartialEq)]
#[data(view)]
struct Viewed {
    name: String,
    n: u32,
}

#[derive(Schema, Debug, PartialEq)]
#[data(view)]
struct ViewedGrown {
    name: String,
    n: u32,
    #[data(default)]
    tags: Vec<String>,
}

#[test]
fn a_view_skips_appended_fields_and_is_none_for_an_older_frame() {
    use substrate_data::frame::read_view;
    let newer = ViewedGrown {
        name: "x".into(),
        n: 7,
        tags: vec!["a".into()],
    };
    let frame = to_frame(&newer);
    let view = read_view::<Viewed>(&frame).unwrap();
    assert_eq!((view.name, view.n), ("x", 7));
    let older = Viewed {
        name: "x".into(),
        n: 7,
    };
    assert!(read_view::<ViewedGrown>(&to_frame(&older)).is_none());
    assert_eq!(
        read_frame::<ViewedGrown>(&to_frame(&older)).unwrap().tags,
        Vec::<String>::new()
    );
}

#[test]
fn a_skipped_field_is_not_a_placeholder() {
    #[derive(Schema, Debug, PartialEq)]
    struct Kept {
        name: String,
        n: u32,
    }
    #[derive(Schema, Debug, PartialEq)]
    struct Skipping {
        name: String,
        #[data(skip)]
        retired: u32,
        n: u32,
    }
    let skipping = Skipping {
        name: "x".into(),
        retired: 9,
        n: 7,
    };
    assert_eq!(
        to_frame(&skipping),
        to_frame(&Kept {
            name: "x".into(),
            n: 7
        })
    );
}

#[test]
fn an_i64_and_a_u64_read_each_others_bits() {
    #[derive(Schema, Debug, PartialEq)]
    struct Signed {
        v: i64,
    }
    #[derive(Schema, Debug, PartialEq)]
    struct Unsigned {
        v: u64,
    }
    let read = read_bytes::<Unsigned>(&to_bytes(&Signed { v: -1 })).unwrap();
    assert_eq!(read.v, u64::MAX);
}
