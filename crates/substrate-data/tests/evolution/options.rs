//! A field that becomes optional, or stops being.

use substrate_data::Schema;

use crate::verdict::Group::Options;
use crate::verdict::{Row, row};

/// Every row of this group.
pub(crate) const ROWS: &[fn() -> Row] = &[
    required_to_option,
    option_to_required_some,
    option_to_required_none,
    text_to_option,
];

#[derive(Schema, Debug, PartialEq)]
struct Needed {
    v: u32,
    after: u8,
}

#[derive(Schema, Debug, PartialEq)]
struct Maybe {
    v: Option<u32>,
    after: u8,
}

fn required_to_option() -> Row {
    row(
        Options,
        "`u32` to `Option<u32>`",
        (
            Needed { v: 7, after: 5 },
            Some(Maybe {
                v: Some(7),
                after: 5,
            }),
        ),
        (
            Maybe {
                v: Some(7),
                after: 5,
            },
            Some(Needed { v: 7, after: 5 }),
        ),
    )
}

fn option_to_required_some() -> Row {
    row(
        Options,
        "`Option<u32>` to `u32`, the old value `Some`",
        (
            Maybe {
                v: Some(7),
                after: 5,
            },
            Some(Needed { v: 7, after: 5 }),
        ),
        (
            Needed { v: 7, after: 5 },
            Some(Maybe {
                v: Some(7),
                after: 5,
            }),
        ),
    )
}

fn option_to_required_none() -> Row {
    row(
        Options,
        "`Option<u32>` to `u32`, the old value `None`",
        (Maybe { v: None, after: 5 }, None),
        (
            Needed { v: 7, after: 5 },
            Some(Maybe {
                v: Some(7),
                after: 5,
            }),
        ),
    )
}

#[derive(Schema, Debug, PartialEq)]
struct Text {
    v: String,
    after: u8,
}

#[derive(Schema, Debug, PartialEq)]
struct MaybeText {
    v: Option<String>,
    after: u8,
}

fn text_to_option() -> Row {
    row(
        Options,
        "`String` to `Option<String>`",
        (
            Text {
                v: "s".into(),
                after: 5,
            },
            Some(MaybeText {
                v: Some("s".into()),
                after: 5,
            }),
        ),
        (
            MaybeText {
                v: Some("s".into()),
                after: 5,
            },
            Some(Text {
                v: "s".into(),
                after: 5,
            }),
        ),
    )
}
