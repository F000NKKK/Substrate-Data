//! The corpus as value trees: the documents typed structs don't describe
//! (map trees, deep nesting, references).

use substrate_data::{Map, Ref, Value};

use super::lcg::Lcg;

fn string(s: impl Into<String>) -> Value {
    Value::Str(s.into())
}

fn reference(kind: &str, id: &str) -> Value {
    Value::Ref(Ref::parse(kind, id).expect("the corpus's references are valid"))
}

fn map<const N: usize>(entries: [(&str, Value); N]) -> Value {
    let mut m = Map::with_capacity(N);
    for (key, value) in entries {
        m.insert(key, value);
    }
    Value::Map(m)
}

/// A config-like tree: settings, users and projects keyed by name; `n` users.
pub(crate) fn config(n: usize) -> Value {
    let mut g = Lcg::new(10);
    let mut users = Map::with_capacity(n);
    let mut projects = Map::with_capacity(n / 4 + 1);
    for i in 0..n {
        let name = format!("{}_{i}", g.name());
        users.insert(
            name.clone(),
            map([
                ("role", string(g.role())),
                ("active", Value::Bool(g.below(4) != 0)),
                ("quota", Value::Float(f64::from(g.below(100) as u32) * 0.5)),
                (
                    "projects",
                    Value::List((0..g.below(3)).map(|_| string(g.folder())).collect()),
                ),
            ]),
        );
        if i % 4 == 0 {
            projects.insert(
                format!("project_{i}"),
                map([
                    ("owner", reference("user", &name)),
                    ("folder", string(g.folder())),
                    ("archived", Value::Bool(false)),
                ]),
            );
        }
    }
    map([
        ("name", string("workspace")),
        (
            "settings",
            map([("theme", string("dark")), ("retries", Value::Int(3))]),
        ),
        ("users", Value::Map(users)),
        ("projects", Value::Map(projects)),
    ])
}

/// A chain `depth` levels deep, each a map; every other level's child sits in a list.
fn chain(depth: usize, level: usize) -> Value {
    if level == depth {
        return Value::Null;
    }
    let next = chain(depth, level + 1);
    let next = if level.is_multiple_of(2) {
        Value::List(vec![next])
    } else {
        next
    };
    map([
        ("level", Value::Int(level as i64)),
        ("name", string("node")),
        ("next", next),
    ])
}

/// `copies` chains `depth` deep.
pub(crate) fn deep(depth: usize, copies: usize) -> Value {
    Value::List((0..copies).map(|_| chain(depth, 0)).collect())
}

/// `n` records whose fields are references: `@file:…`, `@user:…`, `@doc:…`.
pub(crate) fn refs(n: usize) -> Value {
    let mut g = Lcg::new(11);
    Value::List(
        (0..n)
            .map(|i| {
                map([
                    (
                        "file",
                        reference("file", &format!("images/photo_{}.png", i % 64)),
                    ),
                    ("owner", reference("user", g.name())),
                    ("doc", reference("doc", &format!("{i}"))),
                    ("label", string(g.word())),
                ])
            })
            .collect(),
    )
}
