//! Derived types of every shape the derive supports, for the schema and
//! round-trip benches: scalars, enums, references, plain wrappers, field
//! codecs, and documents that name references a resolver checks or expands.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use substrate_data::schema::Dictionary;
use substrate_data::with::{Expand, List, Opt, Plain, RefTo, Spelled, Values};
use substrate_data::{
    Atom, Id, KindName, Map, OneOrList, Ordered, Ref, RefKey, Schema, Uid, Value,
};

use super::lcg::{Lcg, NAMES};

substrate_data::kind!(Account = "account": u64, Page = "page": u64);
substrate_data::kind!(User = "user", Doc = "doc", File = "file");

/// A mood: an enum of unit variants.
#[derive(Schema, Clone, Copy, PartialEq, Debug)]
pub(crate) enum Mood {
    Calm,
    Eager,
}

/// An event: an enum tagged by a field, with data in some variants.
#[derive(Schema, Clone, PartialEq, Debug)]
#[data(tag = "type")]
pub(crate) enum Event {
    Opened { at: u32 },
    Closed,
}

/// A titled weight.
#[derive(Schema, Clone, PartialEq, Debug)]
pub(crate) struct Entry {
    pub(crate) title: String,
    pub(crate) weight: f32,
}

/// One of everything a struct can hold.
#[derive(Schema, Clone, PartialEq, Debug)]
pub(crate) struct Shaped {
    pub(crate) flag: bool,
    pub(crate) count: i16,
    pub(crate) total: u64,
    pub(crate) ratio: f32,
    pub(crate) precise: f64,
    pub(crate) name: String,
    pub(crate) nick: Atom,
    pub(crate) unique: Uid,
    pub(crate) owner: Id<Account>,
    pub(crate) page: Id<Page>,
    pub(crate) anything: Ref,
    pub(crate) mood: Mood,
    pub(crate) event: Event,
    pub(crate) maybe: Option<u8>,
    pub(crate) signed: Vec<i32>,
    pub(crate) entries: Vec<Entry>,
    pub(crate) any: Value,
}

/// A list of [`Shaped`].
#[derive(Schema, Clone, PartialEq, Debug)]
pub(crate) struct Zoo {
    pub(crate) items: Vec<Shaped>,
}

/// The plain types of the standard library a schema reads and writes.
#[derive(Schema, Clone, PartialEq, Debug)]
pub(crate) struct Plains {
    pub(crate) one_or_list: OneOrList<u32>,
    pub(crate) ordered: Ordered<f64>,
    pub(crate) tree: BTreeMap<String, u32>,
    pub(crate) hash: HashMap<String, u32>,
    pub(crate) tuple: (u8, String, f64),
    pub(crate) outcome: Result<u32, String>,
    pub(crate) boxed: Box<u16>,
    pub(crate) array: [f64; 3],
    pub(crate) set: BTreeSet<u16>,
    pub(crate) path: PathBuf,
    pub(crate) cow: Cow<'static, str>,
    pub(crate) shared: Arc<str>,
    pub(crate) boxed_str: Box<str>,
    pub(crate) unit: (),
}

/// A field of each codec of `with`.
#[derive(Schema, Clone, PartialEq, Debug)]
pub(crate) struct Wrapped {
    #[data(with = "RefTo<File>")]
    pub(crate) file: String,
    #[data(with = "Opt<RefTo<User>>")]
    pub(crate) owner: Option<String>,
    #[data(with = "Spelled")]
    pub(crate) spelled: String,
    #[data(with = "List<Spelled>")]
    pub(crate) spellings: Vec<String>,
    #[data(with = "Values<Plain>")]
    pub(crate) by_name: BTreeMap<String, u32>,
    #[data(with = "substrate_data::with::Raw")]
    pub(crate) raw: Vec<u8>,
}

/// A list of [`Wrapped`].
#[derive(Schema, Clone, PartialEq, Debug)]
pub(crate) struct Wrappeds {
    pub(crate) items: Vec<Wrapped>,
}

/// A record of references to a file, a user and a doc.
#[derive(Schema, Clone, PartialEq, Debug)]
pub(crate) struct Attachment {
    pub(crate) file: Id<File>,
    pub(crate) owner: Id<User>,
    pub(crate) doc: Id<Doc>,
    pub(crate) label: String,
}

/// A list of [`Attachment`].
#[derive(Schema, Clone, PartialEq, Debug)]
pub(crate) struct Attachments {
    pub(crate) items: Vec<Attachment>,
}

/// A card whose project is a value, or a reference a dictionary expands.
#[derive(Schema, Clone, PartialEq, Debug)]
pub(crate) struct Card {
    #[data(with = "Expand<Plain>")]
    pub(crate) project: Value,
    pub(crate) count: u32,
}

/// A list of [`Card`].
#[derive(Schema, Clone, PartialEq, Debug)]
pub(crate) struct Cards {
    pub(crate) items: Vec<Card>,
}

/// `n` shaped records.
pub(crate) fn zoo(n: usize) -> Zoo {
    let mut g = Lcg::new(20);
    let items = (0..n)
        .map(|i| Shaped {
            flag: i % 2 == 0,
            count: g.below(2000) as i16 - 1000,
            total: g.next_u64(),
            ratio: g.unit() as f32,
            precise: g.unit(),
            name: g.phrase(1, 3),
            nick: Atom::new(g.name()),
            unique: Uid(u128::from(g.next_u64()) << 64 | u128::from(g.next_u64())),
            owner: Id::new(g.below(1000) as u64),
            page: Id::new(i as u64),
            anything: Ref::parse("file", &format!("images/photo_{}.png", i % 64)).unwrap(),
            mood: if g.below(2) == 0 {
                Mood::Calm
            } else {
                Mood::Eager
            },
            event: if i % 3 == 0 {
                Event::Closed
            } else {
                Event::Opened { at: i as u32 }
            },
            maybe: (i % 4 != 0).then_some(g.below(200) as u8),
            signed: (0..g.below(6))
                .map(|_| g.below(1000) as i32 - 500)
                .collect(),
            entries: (0..g.below(3))
                .map(|_| Entry {
                    title: g.phrase(1, 3),
                    weight: g.below(10) as f32 * 0.5,
                })
                .collect(),
            any: Value::List(vec![Value::Int(i as i64), Value::Str(g.word().into())]),
        })
        .collect();
    Zoo { items }
}

/// One of each plain type.
pub(crate) fn plains() -> Plains {
    Plains {
        one_or_list: OneOrList(vec![1, 2, 3]),
        ordered: Ordered(vec![("first".into(), 1.5), ("second".into(), -2.0)]),
        tree: (0..4).map(|i| (format!("key_{i}"), i)).collect(),
        hash: (0..4).map(|i| (format!("key_{i}"), i)).collect(),
        tuple: (7, "seven".into(), 7.5),
        outcome: Ok(3),
        boxed: Box::new(500),
        array: [1.0, 2.0, 3.0],
        set: [3, 1, 2].into_iter().collect(),
        path: PathBuf::from("projects/alpha/notes.txt"),
        cow: Cow::Borrowed("borrowed"),
        shared: Arc::from("shared"),
        boxed_str: "boxed".into(),
        unit: (),
    }
}

/// `n` records, each a field of every codec of `with`.
pub(crate) fn wrappeds(n: usize) -> Wrappeds {
    let mut g = Lcg::new(21);
    let items = (0..n)
        .map(|i| Wrapped {
            file: format!("images/photo_{}.png", i % 64),
            owner: (i % 3 != 0).then(|| g.name().to_owned()),
            spelled: format!("@file:docs/page_{i}.txt"),
            spellings: vec!["plain".into(), format!("@user:{}", g.name())],
            by_name: (0..3).map(|k| (format!("k{k}"), k)).collect(),
            raw: (0..16).map(|_| g.next_u64() as u8).collect(),
        })
        .collect();
    Wrappeds { items }
}

/// `n` attachments.
pub(crate) fn attachments(n: usize) -> Attachments {
    let mut g = Lcg::new(22);
    let items = (0..n)
        .map(|i| Attachment {
            file: Id::new(format!("images/photo_{}.png", i % 64)),
            owner: Id::new(g.name()),
            doc: Id::new(i.to_string()),
            label: g.word().to_owned(),
        })
        .collect();
    Attachments { items }
}

/// `n` cards, each naming one of sixteen projects.
pub(crate) fn cards(n: usize) -> Cards {
    let items = (0..n)
        .map(|i| Card {
            project: Value::Ref(Ref::parse("project", &format!("p{}", i % 16)).unwrap()),
            count: i as u32,
        })
        .collect();
    Cards { items }
}

/// The files, users and projects the documents above name.
pub(crate) fn dictionary() -> Dictionary {
    let mut d = Dictionary::new();
    let mut put = |kind: &str, id: String, value: Value| {
        d.insert(
            KindName::new(kind).unwrap(),
            RefKey::new(&id).unwrap(),
            value,
        );
    };
    for i in 0..64 {
        let mut m = Map::new();
        m.insert("bytes", Value::Int(i * 1000));
        put("file", format!("images/photo_{i}.png"), Value::Map(m));
    }
    for name in NAMES {
        put("user", name.to_owned(), Value::Str(name.to_uppercase()));
    }
    for i in 0..16 {
        let mut m = Map::new();
        m.insert("name", Value::Str(format!("Project {i}")));
        m.insert(
            "color",
            Value::List(vec![
                Value::Float(0.25),
                Value::Float(0.5),
                Value::Float(1.0),
            ]),
        );
        put("project", format!("p{i}"), Value::Map(m));
    }
    d
}
