//! A sample: one document of the corpus, in the forms the benches start from.

use std::fmt;

use substrate_data::{Schema, Value, binary, text};

use super::typed::{blob, docs, numbers, texts};
use super::values;

/// What a document is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Topic {
    /// A map tree: settings, users and projects keyed by name.
    Config,
    /// A long list of records with repeated keys and strings.
    Records,
    /// Lists of integers and of `f32`-exact and full `f64` floats.
    Numbers,
    /// Long text, non-ASCII characters, escapes.
    Strings,
    /// Nesting, maps in lists in maps.
    Deep,
    /// A list of records made of `@file:…` references.
    Refs,
    /// Bytes as they are.
    Blobs,
}

impl Topic {
    /// Every topic.
    pub(crate) const ALL: [Topic; 7] = [
        Topic::Config,
        Topic::Records,
        Topic::Numbers,
        Topic::Strings,
        Topic::Deep,
        Topic::Refs,
        Topic::Blobs,
    ];
}

impl fmt::Display for Topic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Topic::Config => "config",
            Topic::Records => "records",
            Topic::Numbers => "numbers",
            Topic::Strings => "strings",
            Topic::Deep => "deep",
            Topic::Refs => "refs",
            Topic::Blobs => "blobs",
        })
    }
}

/// How big: about 100 bytes, 5 KB or 500 KB of compact text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Size {
    /// About 100 bytes.
    Small,
    /// About 5 KB.
    Medium,
    /// About 500 KB.
    Large,
}

impl Size {
    /// Every size.
    pub(crate) const ALL: [Size; 3] = [Size::Small, Size::Medium, Size::Large];
}

impl fmt::Display for Size {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Size::Small => "small",
            Size::Medium => "medium",
            Size::Large => "large",
        })
    }
}

/// One document in the forms the benches start from.
#[derive(Debug, Clone)]
pub(crate) struct Sample {
    /// `topic_size`, a benchmark id.
    pub(crate) name: String,
    /// What it is about.
    pub(crate) topic: Topic,
    /// How big.
    pub(crate) size: Size,
    /// The value tree.
    pub(crate) value: Value,
    /// Its compact text.
    pub(crate) text: String,
    /// Its pretty text.
    pub(crate) pretty: String,
    /// Its binary form.
    pub(crate) bytes: Vec<u8>,
}

impl Sample {
    /// The document of `topic` and `size`.
    pub(crate) fn new(topic: Topic, size: Size) -> Self {
        let value = build(topic, size);
        Self {
            name: format!("{topic}_{size}"),
            topic,
            size,
            text: text::to_string(&value),
            pretty: text::to_string_pretty(&value),
            bytes: binary::encode(&value),
            value,
        }
    }
}

/// How many items (users, records, numbers, strings, copies, bytes) of a
/// topic make a document of a size.
fn scale(topic: Topic, size: Size) -> usize {
    let pick = |small, medium, large| match size {
        Size::Small => small,
        Size::Medium => medium,
        Size::Large => large,
    };
    match topic {
        Topic::Config => pick(1, 46, 4700),
        Topic::Records => pick(1, 30, 2900),
        Topic::Numbers => pick(1, 100, 10_000),
        Topic::Strings => pick(1, 36, 3600),
        Topic::Deep => pick(1, 8, 360),
        Topic::Refs => pick(1, 55, 5400),
        Topic::Blobs => pick(48, 3700, 375_000),
    }
}

fn build(topic: Topic, size: Size) -> Value {
    let n = scale(topic, size);
    match topic {
        Topic::Config => values::config(n),
        Topic::Records => docs(n).write(),
        Topic::Numbers => numbers(n).write(),
        Topic::Strings => texts(n).write(),
        Topic::Deep => values::deep(depth(size), n),
        Topic::Refs => values::refs(n),
        Topic::Blobs => blob(n).write(),
    }
}

/// The nesting of a deep document.
fn depth(size: Size) -> usize {
    match size {
        Size::Small => 3,
        Size::Medium => 16,
        Size::Large => 40,
    }
}

/// Every document: each topic in each size.
pub(crate) fn samples() -> Vec<Sample> {
    Topic::ALL
        .into_iter()
        .flat_map(|t| Size::ALL.map(|s| Sample::new(t, s)))
        .collect()
}

/// `text` with a comment above every fifth line that opens a member.
pub(crate) fn commented(text: &str) -> String {
    let mut out = String::from("// a document, commented\n/* its notes\n   on two lines */\n");
    let mut members = 0usize;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('"') {
            members += 1;
            if members.is_multiple_of(5) {
                let pad = &line[..line.len() - trimmed.len()];
                out.push_str(pad);
                out.push_str("// note ");
                out.push_str(&members.to_string());
                out.push('\n');
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// `value` with one more entry at the top: what an editor saves after a small edit.
pub(crate) fn edited(value: &Value) -> Value {
    let mut next = value.clone();
    match &mut next {
        Value::Map(m) => {
            m.insert("edited", Value::Bool(true));
        }
        Value::List(items) => items.push(Value::Int(1)),
        _ => {}
    }
    next
}
