//! Resolvers and references: ids rewritten, refused or missed, references
//! read against the kinds a field takes, and the references a read met.

use substrate_data::schema::{read_id, read_ref};
use substrate_data::{
    Bump, Path, Problem, Reader, Ref, RefKey, Resolve, Schema, Unresolved, Value, from_value,
    from_value_with,
};

substrate_data::kind!(File = "file", Doc = "doc", User = "user": u64);

fn map(entries: &[(&str, Value)]) -> Value {
    Value::Map(
        entries
            .iter()
            .map(|(k, v)| ((*k).to_owned(), v.clone()))
            .collect(),
    )
}

/// What the resolver does with an id it knows.
#[derive(Clone, Copy)]
enum Rule {
    Missing,
    Refused,
    Number,
}

const RULES: [(&str, Rule); 3] = [
    ("gone", Rule::Missing),
    ("locked", Rule::Refused),
    ("number", Rule::Number),
];

/// Rewrites ids, refuses some, misses others.
struct Rewriter;

impl Resolve for Rewriter {
    fn reference(&mut self, r: &Ref, path: &Path) -> Result<RefKey, Unresolved> {
        let id = r.id.as_str();
        let rule = RULES
            .iter()
            .find(|(name, _)| *name == id)
            .map(|(_, rule)| *rule);
        match rule {
            Some(Rule::Missing) => Err(Unresolved::Missing(r.clone())),
            Some(Rule::Refused) => Err(Unresolved::Refused {
                reference: r.clone(),
                reason: format!("not at {path}"),
            }),
            Some(Rule::Number) => Ok(RefKey::new("x1").expect("a key")),
            None => Ok(RefKey::new(&format!("{id}-resolved")).expect("a key")),
        }
    }
}

#[derive(Schema, Debug, PartialEq)]
struct Links {
    file: substrate_data::Id<File>,
    user: Option<substrate_data::Id<User>>,
}

#[test]
fn a_resolver_rewrites_refuses_and_misses_references() {
    let ok = map(&[("file", "@file:a".into())]);
    let read = from_value_with::<Links>(&ok, &mut Rewriter).unwrap();
    assert_eq!(read.value.file.id, "a-resolved");
    let bad = map(&[("file", "@file:gone".into()), ("user", "@user:1".into())]);
    let problems: Vec<_> = from_value_with::<Links>(&bad, &mut Rewriter)
        .unwrap_err()
        .iter()
        .map(|d| (d.path.to_string(), d.message.clone(), d.code))
        .collect();
    assert_eq!(problems[0].1, "`@file:gone`: there is no such file");
    assert_eq!(problems[0].2, Some(Problem::Unresolved.into()));
    // The resolver gave an id that is not a user's number.
    assert_eq!(problems[1].0, "user");
    assert!(problems[1].1.contains("isn't a user id"), "{problems:?}");
    let locked = map(&[("file", "@file:locked".into())]);
    let e = from_value_with::<Links>(&locked, &mut Rewriter).unwrap_err();
    assert_eq!(e[0].message, "`@file:locked`: not at file");
    let missing = Unresolved::Missing(Ref::parse("doc", "d").unwrap());
    assert_eq!(missing.to_string(), "`@doc:d`: there is no such doc");
    let as_error: &dyn std::error::Error = &missing;
    assert_ne!(as_error.to_string(), "");
}

#[test]
fn a_reader_resolves_without_a_resolver_to_the_id_as_written() {
    let mut cx = Reader::new();
    let r = Ref::parse("file", "a").unwrap();
    assert!(!cx.resolves() && cx.keeps_found());
    assert_eq!(cx.resolve_ref(&r).unwrap().as_str(), "a");
    assert_eq!(cx.take_ref(r.clone()).unwrap().as_str(), "a");
    cx.note_ref(r.clone());
    assert_eq!(cx.refs.len(), 1);
    let mut rewriter = Rewriter;
    let mut with = Reader::with_resolver(&mut rewriter);
    assert!(with.resolves());
    assert_eq!(with.take_ref(r.clone()).unwrap().as_str(), "a-resolved");
    assert!(format!("{with:?}").contains("resolves: true"));
}

#[test]
fn references_are_read_against_the_kinds_a_field_takes() {
    let arena = Bump::new();
    let cases = [
        (Value::from("@file:a"), None),
        (
            Value::from("@doc:a"),
            Some("`@doc:a` is a reference to doc — this takes `@file:` or `@user:`"),
        ),
        (
            Value::from("a"),
            Some("`a`: name it by a reference — `@file:a`"),
        ),
        (
            Value::Int(1),
            Some("expected a reference `@kind:id`, found 1"),
        ),
        (
            Value::from("@bytes:AQI="),
            Some("expected a reference (`@file:` or `@user:`), found bytes"),
        ),
        (
            Value::from("@@x"),
            Some("expected a reference (`@file:` or `@user:`), found a string"),
        ),
        (
            Value::from("@Bad:x"),
            Some(
                "`Bad` isn't a kind (lower case letters, digits and `_`: `user`, `doc`, `file`, ...)",
            ),
        ),
        (Value::Ref(Ref::parse("file", "b").unwrap()), None),
    ];
    for (v, want) in cases {
        let mut cx = Reader::new();
        let got = read_ref(&v.borrow(&arena), &mut cx, &["file", "user"]);
        match want {
            None => assert!(got.is_some() && cx.problems.is_empty(), "{v:?}"),
            Some(w) => {
                assert!(got.is_none(), "{v:?}");
                assert_eq!(cx.problems[0].message, w);
            }
        }
    }
}

#[test]
fn a_reference_with_no_kinds_named_takes_any_kind() {
    let arena = Bump::new();
    let mut bare = Reader::new();
    assert!(read_ref(&Value::from("plain").borrow(&arena), &mut bare, &[]).is_none());
    assert_eq!(
        bare.problems[0].message,
        "`plain`: name it by a reference — `@kind:plain`"
    );
    let mut any = Reader::new();
    let found = read_ref(&Value::from("@doc:a").borrow(&arena), &mut any, &[]).expect("a ref");
    assert_eq!(found.to_string(), "@doc:a");
    assert_eq!(any.refs.len(), 1);
    let mut typed = Reader::new();
    assert_eq!(
        read_id::<(File, Doc)>(&Value::from("@doc:q").borrow(&arena), &mut typed).as_deref(),
        Some("q")
    );
}

#[test]
fn reading_keeps_the_references_it_met_with_their_places() {
    let v = map(&[("file", "@file:a".into()), ("user", "@user:9".into())]);
    let read = from_value::<Links>(&v).unwrap();
    let found: Vec<_> = read
        .refs
        .iter()
        .map(|f| format!("{} {}", f.path, f.value))
        .collect();
    assert_eq!(found, ["file @file:a", "user @user:9"]);
    let only = substrate_data::read_ref_value::<Links>(&v.borrow(&Bump::new())).unwrap();
    assert_eq!(only.user.map(|u| u.id), Some(9));
}
