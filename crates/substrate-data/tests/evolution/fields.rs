//! Fields added, removed, moved and renamed.

use substrate_data::Schema;

use crate::base::{base, grown};
use crate::verdict::Group::Fields;
use crate::verdict::{Row, row};

/// Every row of this group.
pub(crate) const ROWS: &[fn() -> Row] = &[
    append_default,
    append_option,
    append_required,
    insert_before,
    remove_last,
    remove_first,
    reorder_ints,
    reorder_mixed,
    rename_keeping_key,
    rename_new_key,
];

fn append_default() -> Row {
    row(
        Fields,
        "append a field with `default`",
        (base(), Some(grown(&[]))),
        (grown(&["a"]), Some(base())),
    )
}

fn append_option() -> Row {
    #[derive(Schema, Debug, PartialEq)]
    struct New {
        name: String,
        n: u32,
        note: Option<String>,
    }
    let new = |note: Option<&str>| New {
        name: "x".into(),
        n: 7,
        note: note.map(str::to_owned),
    };
    row(
        Fields,
        "append an `Option` field",
        (base(), Some(new(None))),
        (new(Some("a")), Some(base())),
    )
}

fn append_required() -> Row {
    #[derive(Schema, Debug, PartialEq)]
    struct New {
        name: String,
        n: u32,
        level: u32,
    }
    let new = New {
        name: "x".into(),
        n: 7,
        level: 3,
    };
    row(
        Fields,
        "append a required field (no `default`, not `Option`)",
        (base(), None),
        (new, Some(base())),
    )
}

fn insert_before() -> Row {
    #[derive(Schema, Debug, PartialEq)]
    struct New {
        name: String,
        #[data(default)]
        level: u32,
        n: u32,
    }
    let new = |level| New {
        name: "x".into(),
        level,
        n: 7,
    };
    row(
        Fields,
        "insert a field with `default` before the last",
        (base(), Some(new(0))),
        (new(3), Some(base())),
    )
}

fn remove_last() -> Row {
    #[derive(Schema, Debug, PartialEq)]
    struct New {
        name: String,
    }
    let new = || New { name: "x".into() };
    row(
        Fields,
        "remove the last field",
        (base(), Some(new())),
        (new(), None),
    )
}

fn remove_first() -> Row {
    #[derive(Schema, Debug, PartialEq)]
    struct New {
        n: u32,
    }
    row(
        Fields,
        "remove the first field",
        (base(), Some(New { n: 7 })),
        (New { n: 7 }, None),
    )
}

#[derive(Schema, Debug, PartialEq)]
struct Pair {
    a: u32,
    b: u32,
}

#[derive(Schema, Debug, PartialEq)]
struct Flipped {
    b: u32,
    a: u32,
}

fn reorder_ints() -> Row {
    row(
        Fields,
        "swap two fields of one type",
        (Pair { a: 1, b: 2 }, Some(Flipped { b: 2, a: 1 })),
        (Flipped { b: 2, a: 1 }, Some(Pair { a: 1, b: 2 })),
    )
}

fn reorder_mixed() -> Row {
    #[derive(Schema, Debug, PartialEq)]
    struct New {
        n: u32,
        name: String,
    }
    let new = || New {
        n: 7,
        name: "x".into(),
    };
    row(
        Fields,
        "swap two fields of different types",
        (base(), Some(new())),
        (new(), Some(base())),
    )
}

fn rename_keeping_key() -> Row {
    #[derive(Schema, Debug, PartialEq)]
    struct New {
        name: String,
        #[data(rename = "n")]
        count: u32,
    }
    let new = || New {
        name: "x".into(),
        count: 7,
    };
    row(
        Fields,
        "rename a field in Rust, keeping its key (`rename`)",
        (base(), Some(new())),
        (new(), Some(base())),
    )
}

fn rename_new_key() -> Row {
    #[derive(Schema, Debug, PartialEq)]
    struct New {
        name: String,
        count: u32,
    }
    let new = || New {
        name: "x".into(),
        count: 7,
    };
    row(
        Fields,
        "rename a field and its key",
        (base(), Some(new())),
        (new(), Some(base())),
    )
}
