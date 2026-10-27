//! The corpus as typed structs: each derives `Schema` and serde's traits, so
//! one value goes through this crate's forms and through every other format.

use std::fmt::Debug;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use substrate_data::Schema;

use super::lcg::Lcg;
use super::sample::Size;

/// A user of a workspace.
#[derive(Schema, Serialize, Deserialize, Clone, PartialEq, Debug)]
#[data(view)]
pub(crate) struct User {
    pub(crate) id: u32,
    pub(crate) name: String,
    pub(crate) email: String,
    pub(crate) role: String,
    pub(crate) active: bool,
    pub(crate) quota: f64,
    pub(crate) projects: Vec<String>,
}

/// Users, as a config-like document.
#[derive(Schema, Serialize, Deserialize, Clone, PartialEq, Debug)]
#[data(view)]
pub(crate) struct Users {
    pub(crate) workspace: String,
    pub(crate) users: Vec<User>,
}

/// A document of a project: keys and strings that repeat from record to record.
#[derive(Schema, Serialize, Deserialize, Clone, PartialEq, Debug)]
#[data(view)]
pub(crate) struct DocRecord {
    pub(crate) id: u32,
    pub(crate) title: String,
    pub(crate) author: String,
    pub(crate) folder: String,
    pub(crate) size: u64,
    pub(crate) modified: u64,
    pub(crate) public: bool,
    pub(crate) tags: Vec<String>,
    pub(crate) rating: f64,
}

/// A long list of records.
#[derive(Schema, Serialize, Deserialize, Clone, PartialEq, Debug)]
#[data(view)]
pub(crate) struct Docs {
    pub(crate) docs: Vec<DocRecord>,
}

/// A file in a listing.
#[derive(Schema, Serialize, Deserialize, Clone, PartialEq, Debug)]
#[data(view)]
pub(crate) struct FileEntry {
    pub(crate) path: String,
    pub(crate) mime: String,
    pub(crate) size: u64,
    pub(crate) owner: String,
}

/// A file listing.
#[derive(Schema, Serialize, Deserialize, Clone, PartialEq, Debug)]
#[data(view)]
pub(crate) struct Files {
    pub(crate) files: Vec<FileEntry>,
}

/// Lists of numbers: counters, ids, signed deltas, `f32`-exact and full `f64` floats.
#[derive(Schema, Serialize, Deserialize, Clone, PartialEq, Debug)]
#[data(view)]
pub(crate) struct Numbers {
    pub(crate) ids: Vec<u32>,
    pub(crate) counters: Vec<u16>,
    pub(crate) deltas: Vec<i32>,
    pub(crate) singles: Vec<f32>,
    pub(crate) doubles: Vec<f64>,
}

/// Long and non-ASCII strings.
#[derive(Schema, Serialize, Deserialize, Clone, PartialEq, Debug)]
#[data(view)]
pub(crate) struct Texts {
    pub(crate) items: Vec<String>,
}

/// A name and bytes as they are.
#[derive(Schema, Serialize, Deserialize, Clone, PartialEq, Debug)]
#[data(view)]
pub(crate) struct Blob {
    pub(crate) name: String,
    #[data(with = "substrate_data::with::Raw")]
    #[serde(with = "serde_bytes")]
    pub(crate) bytes: Vec<u8>,
}

/// `n` users.
pub(crate) fn users(n: usize) -> Users {
    let mut g = Lcg::new(1);
    let users = (0..n)
        .map(|i| {
            let name = g.name();
            User {
                id: i as u32,
                name: format!("{name}_{i}"),
                email: format!("{name}{i}@example.org"),
                role: g.role().to_owned(),
                active: g.below(4) != 0,
                quota: f64::from(g.below(100) as u32) * 0.5,
                projects: (0..g.below(4)).map(|_| g.folder().to_owned()).collect(),
            }
        })
        .collect();
    Users {
        workspace: "main".to_owned(),
        users,
    }
}

/// `n` docs.
pub(crate) fn docs(n: usize) -> Docs {
    let mut g = Lcg::new(2);
    let docs = (0..n)
        .map(|i| DocRecord {
            id: i as u32,
            title: g.phrase(2, 4),
            author: g.name().to_owned(),
            folder: g.folder().to_owned(),
            size: g.below(2_000_000) as u64,
            modified: 1_700_000_000 + g.below(30_000_000) as u64,
            public: g.below(3) == 0,
            tags: (0..g.below(4)).map(|_| g.tag().to_owned()).collect(),
            rating: f64::from(g.below(50) as u32) / 10.0,
        })
        .collect();
    Docs { docs }
}

/// `n` files.
pub(crate) fn files(n: usize) -> Files {
    let mut g = Lcg::new(3);
    let files = (0..n)
        .map(|i| {
            let (ext, mime) = *g.pick(&[
                ("png", "image/png"),
                ("jpg", "image/jpeg"),
                ("wav", "audio/wav"),
                ("txt", "text/plain"),
            ]);
            FileEntry {
                path: format!("{}/{}_{i}.{ext}", g.folder(), g.word()),
                mime: mime.to_owned(),
                size: g.below(5_000_000) as u64,
                owner: g.name().to_owned(),
            }
        })
        .collect();
    Files { files }
}

/// `n` numbers in each list.
pub(crate) fn numbers(n: usize) -> Numbers {
    let mut g = Lcg::new(4);
    let mut id = 1000u32;
    Numbers {
        ids: (0..n)
            .map(|_| {
                id += 1 + g.below(9) as u32;
                id
            })
            .collect(),
        counters: (0..n).map(|_| g.below(300) as u16).collect(),
        deltas: (0..n).map(|_| g.below(2001) as i32 - 1000).collect(),
        singles: (0..n).map(|_| (g.unit() * 100.0) as f32).collect(),
        doubles: (0..n).map(|_| g.unit() * 1e6 - 5e5).collect(),
    }
}

/// `n` strings: paragraphs, non-ASCII, escapes.
pub(crate) fn texts(n: usize) -> Texts {
    let mut g = Lcg::new(5);
    let items = (0..n)
        .map(|i| match (i + 1) % 3 {
            0 => g.phrase(20, 40),
            1 => format!("{} {}", g.unicode(), g.words(5)),
            _ => format!("@{} says \"{}\"", g.name(), g.words(8)),
        })
        .collect();
    Texts { items }
}

/// A blob of `n` pseudo-random bytes.
pub(crate) fn blob(n: usize) -> Blob {
    let mut g = Lcg::new(6);
    Blob {
        name: "picture.png".to_owned(),
        bytes: (0..n).map(|_| g.next_u64() as u8).collect(),
    }
}

/// `n` users as TOML: `[[users]]` tables.
#[cfg(feature = "toml")]
pub(crate) fn toml_users(n: usize) -> String {
    use std::fmt::Write as _;
    let mut out = String::from("workspace = \"main\"\n");
    for u in users(n).users {
        write!(
            out,
            "\n[[users]]\nid = {}\nname = \"{}\"\nemail = \"{}\"\nrole = \"{}\"\nactive = {}\nquota = {:?}\nprojects = {:?}\n",
            u.id, u.name, u.email, u.role, u.active, u.quota, u.projects
        )
        .expect("a String takes writes");
    }
    out
}

/// What a typed document is measured as: the bounds the visitor gets.
pub(crate) trait Typed: Schema + Serialize + DeserializeOwned + PartialEq + Debug {}

impl<T: Schema + Serialize + DeserializeOwned + PartialEq + Debug> Typed for T {}

/// Something to do with each typed document in turn.
pub(crate) trait Visit {
    /// Called with `name` (`docs_large`) and the document.
    fn visit<T: Typed>(&mut self, name: &str, value: &T);
}

/// How many items of each typed document a size has.
struct Counts {
    users: usize,
    docs: usize,
    files: usize,
    numbers: usize,
    texts: usize,
    bytes: usize,
}

const fn counts(size: Size) -> Counts {
    match size {
        Size::Small => Counts {
            users: 1,
            docs: 1,
            files: 1,
            numbers: 1,
            texts: 1,
            bytes: 48,
        },
        Size::Medium => Counts {
            users: 40,
            docs: 30,
            files: 50,
            numbers: 100,
            texts: 36,
            bytes: 3700,
        },
        Size::Large => Counts {
            users: 4000,
            docs: 2900,
            files: 5000,
            numbers: 10_000,
            texts: 3600,
            bytes: 375_000,
        },
    }
}

/// Every typed document of each of `sizes`, given to `v`.
pub(crate) fn each(v: &mut impl Visit, sizes: &[Size]) {
    for &size in sizes {
        let n = counts(size);
        v.visit(&format!("users_{size}"), &users(n.users));
        v.visit(&format!("docs_{size}"), &docs(n.docs));
        v.visit(&format!("files_{size}"), &files(n.files));
        v.visit(&format!("numbers_{size}"), &numbers(n.numbers));
        v.visit(&format!("texts_{size}"), &texts(n.texts));
        v.visit(&format!("blob_{size}"), &blob(n.bytes));
    }
}
