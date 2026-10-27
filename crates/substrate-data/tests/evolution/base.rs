//! The record most changes start from, and its grown form.

use substrate_data::Schema;

/// The first version.
#[derive(Schema, Debug, PartialEq)]
pub(crate) struct Base {
    pub(crate) name: String,
    pub(crate) n: u32,
}

/// `Base` with a field appended (`default`).
#[derive(Schema, Debug, PartialEq)]
pub(crate) struct Grown {
    pub(crate) name: String,
    pub(crate) n: u32,
    #[data(default)]
    pub(crate) tags: Vec<String>,
}

/// A record in a field, with a field after it.
#[derive(Schema, Debug, PartialEq)]
pub(crate) struct In<T: Schema> {
    pub(crate) v: T,
    pub(crate) after: u8,
}

pub(crate) fn base() -> Base {
    Base {
        name: "x".into(),
        n: 7,
    }
}

pub(crate) fn grown(tags: &[&str]) -> Grown {
    Grown {
        name: "x".into(),
        n: 7,
        tags: tags.iter().map(|t| (*t).to_owned()).collect(),
    }
}
