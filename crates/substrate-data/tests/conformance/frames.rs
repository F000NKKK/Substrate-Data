//! Frame cases: records, strings, options, references, enums and floats.

use substrate_data::{Id, Ref};

use crate::case::Case;
use crate::schemas::{
    Activity, Blob, Event, Folder, Links, Member, Options, Page, Role, Samples, Scalars,
};
use crate::typed::typed_case;

/// Every case of this module.
pub(crate) fn cases() -> Vec<Case> {
    let mut all = records();
    all.extend(options());
    all.extend(strings());
    all.extend(references());
    all.extend(folder());
    all.extend(events());
    all.extend(samples());
    all
}

fn records() -> Vec<Case> {
    let plain = Scalars {
        flag: true,
        count: -300,
        big: 4_000_000_000,
        ratio: 0.25,
        precise: 0.1,
        name: "alice".into(),
    };
    let extremes = Scalars {
        flag: false,
        count: i32::MIN,
        big: i64::MAX.cast_unsigned(),
        ratio: -0.0,
        precise: f64::MIN_POSITIVE,
        name: String::new(),
    };
    vec![
        typed_case(
            "frame_scalars",
            "A record: its byte length, then the fields in declaration order: a bool byte, zigzag varints for ints, f32 and f64 raw little-endian, a string as its table index.",
            "scalars",
            &plain,
        ),
        typed_case(
            "frame_scalars_extremes",
            "Extremes: i32::MIN, i64::MAX as an unsigned number, -0.0 as an f32, an empty string.",
            "scalars",
            &extremes,
        ),
    ]
}

fn options() -> Vec<Case> {
    let options = Options {
        count: Some(7),
        note: None,
        missing: Some(0),
    };
    let absent = Options {
        count: None,
        note: Some("n".into()),
        missing: None,
    };
    vec![
        typed_case(
            "frame_options_present",
            "An Option is a byte 0 or 1, then the value when there is one.",
            "options",
            &options,
        ),
        typed_case(
            "frame_options_absent",
            "A missing Option is only the byte 0.",
            "options",
            &absent,
        ),
        typed_case(
            "frame_bytes",
            "Bytes: a varint length, then them raw; the string next to them is a table index.",
            "blob",
            &Blob {
                name: "logo".into(),
                data: vec![0, 1, 2, 255],
            },
        ),
    ]
}

fn strings() -> Vec<Case> {
    let shared = Page {
        title: "Intro".into(),
        author: "alice".into(),
        editor: "alice".into(),
        tags: vec!["a".into(), "b".into(), "a".into(), "Intro".into()],
    };
    let empty = Page {
        title: String::new(),
        author: String::new(),
        editor: "ü".into(),
        tags: vec![],
    };
    let long = Page {
        title: "t".repeat(130),
        author: "alice".into(),
        editor: "alice".into(),
        tags: vec![],
    };
    vec![
        typed_case(
            "frame_strings_shared",
            "Strings go in a table after the body, each once, in the order first used; the body holds table indexes. A repeated string is the same index.",
            "page",
            &shared,
        ),
        typed_case(
            "frame_strings_empty_and_unicode",
            "The empty string is a table entry; the table holds raw UTF-8.",
            "page",
            &empty,
        ),
        typed_case(
            "frame_table_length_two_bytes",
            "A 130-byte string makes the table longer than 127 bytes: its length (131 with its own length byte) is a two-byte varint written backwards at the frame's end.",
            "page",
            &long,
        ),
    ]
}

fn references() -> Vec<Case> {
    let links = Links {
        owner: Id::new(7u64),
        parent: Id::new("guides/intro"),
        attachment: Ref::parse("file", "images/logo.png").expect("a reference"),
    };
    let numbers = Links {
        owner: Id::new(300u64),
        parent: Id::new("x"),
        attachment: Ref::parse("user", "alice").expect("a reference"),
    };
    vec![
        typed_case(
            "frame_links",
            "An `Id<User>` packs as its id's own type (a u64: one varint) with the kind implied by the field; an `Id<Doc>` as a table string; a `Ref` of any kind as two table strings, kind and id.",
            "links",
            &links,
        ),
        typed_case(
            "frame_links_wide_ids",
            "A user id of 300 takes a two-byte varint.",
            "links",
            &numbers,
        ),
    ]
}

fn folder() -> Vec<Case> {
    let folder = Folder {
        name: "team".into(),
        members: vec![
            Member {
                name: "alice".into(),
                role: Role::Admin,
            },
            Member {
                name: "bob".into(),
                role: Role::Reader,
            },
            Member {
                name: "carol".into(),
                role: Role::Editor,
            },
        ],
        tags: vec!["x".into()],
    };
    vec![typed_case(
        "frame_folder",
        "A list of records: a count, the list's byte length, then each record with its own length; a unit enum is its variant index as a varint.",
        "folder",
        &folder,
    )]
}

fn events() -> Vec<Case> {
    let opened = Activity {
        id: 1,
        event: Event::Opened { at: 9 },
    };
    let renamed = Activity {
        id: 2,
        event: Event::Renamed {
            name: "notes".into(),
        },
    };
    let closed = Activity {
        id: 3,
        event: Event::Closed,
    };
    vec![
        typed_case(
            "frame_event_with_fields",
            "A tagged enum is not packed: it is held as its binary form (no header, its own string table) behind a length, and read as a value.",
            "activity",
            &opened,
        ),
        typed_case(
            "frame_event_with_string",
            "The held binary form has its own string table: `notes` is inside it, not in the frame's table.",
            "activity",
            &renamed,
        ),
        typed_case(
            "frame_event_unit",
            "A tagged variant without fields is a map holding only the tag.",
            "activity",
            &closed,
        ),
    ]
}

fn samples() -> Vec<Case> {
    let full = Samples {
        narrow: vec![0.5, 1.5],
        exact: vec![0.5, 0.25],
        wide: vec![0.1, 0.2],
        signed: vec![-1, 0, 1, -2, 70_000],
        unsigned: vec![1, 2, 65_535],
    };
    vec![
        typed_case(
            "frame_samples",
            "Lists of numbers: f32s raw (4 bytes each); f64s with a flag byte, 1 when every one is an f32 exactly (4 bytes each) else 0 (8 bytes); ints bit-packed with a count and a width byte, signed ones zigzagged first.",
            "samples",
            &full,
        ),
    ]
    .into_iter()
    .chain(empty_and_long())
    .collect()
}

fn empty_and_long() -> Vec<Case> {
    let empty = Samples {
        narrow: vec![],
        exact: vec![],
        wide: vec![],
        signed: vec![],
        unsigned: vec![],
    };
    let long = Samples {
        wide: (0..20).map(|i| f64::from(i) + 0.1).collect(),
        ..empty.clone()
    };
    vec![
        typed_case(
            "frame_samples_empty",
            "Empty lists: a count 0; an empty f64 list has the flag 1; an empty int list has width 1.",
            "samples",
            &empty,
        ),
        typed_case(
            "frame_record_length_two_bytes",
            "A record longer than 127 bytes has a two-byte varint length.",
            "samples",
            &long,
        ),
    ]
}
