//! References expanded into values: a field that takes a value or a name
//! for one, read through a resolver (a dictionary or the tool's own).

use substrate_data::schema::{Dictionary, from_bytes_with};
use substrate_data::with::{Expand, List, Opt, Plain};
use substrate_data::{
    Code, Diagnostic, Path, Problem, Ref, RefKey, Resolve, Schema, Unresolved, Value,
    from_doc_with, from_text, from_value_with, read_bytes, to_bytes,
};

fn problem(d: &Diagnostic) -> Option<Problem> {
    match d.code {
        Some(Code::Data(p)) => Some(p),
        _ => None,
    }
}

#[derive(Schema, Debug, PartialEq)]
struct Notice {
    #[data(with = "Expand<Plain>")]
    title: String,
    #[data(with = "Opt<Expand<Plain>>", default)]
    subtitle: Option<String>,
    #[data(with = "List<Expand<Plain>>", default)]
    tags: Vec<String>,
}

#[derive(Schema, Debug, PartialEq)]
struct Profile {
    name: String,
    age: u32,
}

#[derive(Schema, Debug, PartialEq)]
struct Card {
    #[data(with = "Expand<Plain>")]
    profile: Profile,
}

const WORDS: &str = r#"{
    word: { hello: "Hello", bye: "Goodbye", at: "@@everyone" },
    user: { ann: { name: "Ann", age: 31 }, bad: { name: "Bad" } },
}"#;

fn dictionary() -> Dictionary {
    from_text::<Dictionary>(WORDS).expect("a dictionary").value
}

fn notice(text: &str, words: &mut Dictionary) -> Result<Notice, Vec<(String, Problem)>> {
    let doc = substrate_data::text::parse(text).expect("syntax");
    from_doc_with::<Notice>(text, &doc, words)
        .map(|r| r.value)
        .map_err(|e| {
            e.0.iter()
                .map(|found| (found.path.to_string(), problem(found).expect("ours")))
                .collect()
        })
}

#[test]
fn a_reference_expands_to_a_string() {
    let n = notice(r#"{ title: "@word:hello" }"#, &mut dictionary()).unwrap();
    assert_eq!(n.title, "Hello");
    // The word is a string as spelled: its doubled `@` reads back as one.
    let escaped = notice(r#"{ title: "@word:at" }"#, &mut dictionary()).unwrap();
    assert_eq!(escaped.title, "@everyone");
}

#[test]
fn a_reference_value_expands_too() {
    let v = Value::Map(
        [("title", Value::Ref(Ref::parse("word", "bye").unwrap()))]
            .into_iter()
            .collect(),
    );
    let n = from_value_with::<Notice>(&v, &mut dictionary()).unwrap();
    assert_eq!(n.value.title, "Goodbye");
    assert_eq!(n.refs.len(), 1);
    assert_eq!(n.refs[0].path.to_string(), "title");
}

#[test]
fn a_reference_expands_to_a_map() {
    let profile = r#"{ profile: "@user:ann" }"#;
    let doc = substrate_data::text::parse(profile).unwrap();
    let c = from_doc_with::<Card>(profile, &doc, &mut dictionary()).unwrap();
    assert_eq!(
        c.value.profile,
        Profile {
            name: "Ann".into(),
            age: 31
        }
    );
}

#[test]
fn a_problem_in_the_expanded_value_is_at_the_reference() {
    let text = r#"{ profile: "@user:bad" }"#;
    let doc = substrate_data::text::parse(text).unwrap();
    let e = from_doc_with::<Card>(text, &doc, &mut dictionary()).unwrap_err();
    assert_eq!(e.0.len(), 1);
    assert_eq!(problem(&e.0[0]), Some(Problem::MissingField));
    assert_eq!(e.0[0].path.to_string(), "profile");
    assert_eq!((e.0[0].line, e.0[0].col), (Some(1), Some(12)));
}

#[test]
fn a_missing_id_is_unresolved_at_its_path() {
    let e = notice(r#"{ title: "@word:nope" }"#, &mut dictionary()).unwrap_err();
    assert_eq!(e, vec![("title".to_owned(), Problem::Unresolved)]);
    let text = r#"{ title: "@word:nope" }"#;
    let doc = substrate_data::text::parse(text).unwrap();
    let err = from_doc_with::<Notice>(text, &doc, &mut dictionary()).unwrap_err();
    assert_eq!(err.0[0].message, "`@word:nope`: there is no such word");
    assert_eq!(err.0[0].code.map(Code::as_str), Some("SD0011"));
}

#[test]
fn inline_values_are_untouched() {
    let n = notice(r#"{ title: "Hi", subtitle: "There" }"#, &mut dictionary()).unwrap();
    assert_eq!(
        (n.title.as_str(), n.subtitle.as_deref()),
        ("Hi", Some("There"))
    );
    // Nothing to resolve with: read as written, a plain string.
    let plain: Notice = from_text(r#"{ title: "Hi" }"#).unwrap().value;
    assert_eq!(plain.title, "Hi");
}

#[test]
fn a_kind_the_dictionary_does_not_hold_is_kept() {
    // Kept as written: the string's own schema reads it as text it isn't.
    let e = notice(r#"{ title: "@file:a" }"#, &mut dictionary()).unwrap_err();
    assert_eq!(e[0].0, "title");
}

#[test]
fn nested_lists_and_options_expand_each_item() {
    let text =
        r#"{ title: "x", subtitle: "@word:bye", tags: ["@word:hello", "plain", "@word:bye"] }"#;
    let n = notice(text, &mut dictionary()).unwrap();
    assert_eq!(n.subtitle.as_deref(), Some("Goodbye"));
    assert_eq!(n.tags, ["Hello", "plain", "Goodbye"]);
    let missing = r#"{ title: "x", tags: ["@word:hello", "@word:no", "@word:also"] }"#;
    let e = notice(missing, &mut dictionary()).unwrap_err();
    assert_eq!(
        e,
        vec![
            ("tags[1]".to_owned(), Problem::Unresolved),
            ("tags[2]".to_owned(), Problem::Unresolved),
        ]
    );
}

/// Refuses every reference, or keeps them.
struct Refuser {
    keep: bool,
}

impl Resolve for Refuser {
    fn reference(&mut self, r: &Ref, _: &Path) -> Result<RefKey, Unresolved> {
        Ok(r.id.clone())
    }
    fn expand(&mut self, r: &Ref, path: &Path) -> Result<Option<Value>, Unresolved> {
        if self.keep {
            return Ok(None);
        }
        Err(Unresolved::Refused {
            reference: r.clone(),
            reason: format!("not allowed at {path}"),
        })
    }
}

#[test]
fn a_resolver_may_refuse_or_keep() {
    let text = r#"{ title: "@word:hello" }"#;
    let doc = substrate_data::text::parse(text).unwrap();
    let e = from_doc_with::<Notice>(text, &doc, &mut Refuser { keep: false }).unwrap_err();
    assert_eq!(e.0[0].message, "`@word:hello`: not allowed at title");
    assert_eq!(problem(&e.0[0]), Some(Problem::Unresolved));
    // Kept: the field's own codec reads what was written.
    let kept = from_doc_with::<Notice>(text, &doc, &mut Refuser { keep: true });
    assert_eq!(problem(&kept.unwrap_err().0[0]), Some(Problem::Invalid));
}

/// A word that expands to a reference to itself.
struct Loop;

impl Resolve for Loop {
    fn reference(&mut self, r: &Ref, _: &Path) -> Result<RefKey, Unresolved> {
        Ok(r.id.clone())
    }
    fn expand(&mut self, r: &Ref, _: &Path) -> Result<Option<Value>, Unresolved> {
        Ok(Some(Value::Map(
            [("next", Value::Ref(r.clone()))].into_iter().collect(),
        )))
    }
}

#[derive(Schema, Debug)]
struct Node {
    #[data(with = "Opt<Expand<Plain>>", default)]
    next: Option<Box<Node>>,
}

#[test]
fn expansion_that_never_ends_is_a_problem() {
    let text = r#"{ next: "@node:a" }"#;
    let doc = substrate_data::text::parse(text).unwrap();
    let e = from_doc_with::<Node>(text, &doc, &mut Loop).unwrap_err();
    assert_eq!(e.0.len(), 1);
    assert!(e.0[0].message.contains("more than 32 deep"), "{:?}", e.0[0]);
}

#[test]
fn a_dictionary_reads_from_text_and_binary_alike() {
    let from_text = dictionary();
    let value: Value = from_text.write();
    let bytes = to_bytes(&from_text);
    let from_binary: Dictionary = read_bytes(&bytes).unwrap();
    assert_eq!(from_binary, from_text);
    assert_eq!(
        from_value_with::<Dictionary>(&value, &mut Dictionary::new())
            .unwrap()
            .value,
        from_text
    );
    assert_eq!(from_text.len(), 5);
    assert!(!from_text.is_empty());
    assert_eq!(
        from_text.kinds().map(|k| k.to_string()).collect::<Vec<_>>(),
        ["user", "word"]
    );
    let ids: Vec<_> = from_text
        .entries(substrate_data::KindName::new("word").unwrap())
        .map(|(id, _)| id.as_str())
        .collect();
    assert_eq!(ids, ["hello", "bye", "at"], "in the order written");
    assert_eq!(
        from_text
            .entries(substrate_data::KindName::new("file").unwrap())
            .count(),
        0
    );
}

#[test]
fn binary_reads_expand_through_a_resolver() {
    let bytes = to_bytes(&Value::Map(
        [("title", Value::Ref(Ref::parse("word", "hello").unwrap()))]
            .into_iter()
            .collect(),
    ));
    let n = from_bytes_with::<Notice>(&bytes, &mut dictionary()).unwrap();
    assert_eq!(n.value.title, "Hello");
    assert_eq!(n.refs.len(), 1);
    let e = from_bytes_with::<Notice>(b"nope", &mut dictionary()).unwrap_err();
    assert_eq!(problem(&e[0]), Some(Problem::BadBinary));
}

#[test]
fn a_malformed_dictionary_has_problems_with_paths() {
    let e = from_text::<Dictionary>(r#"{ Word: {}, user: 3, file: { "a b": 1 } }"#).unwrap_err();
    let got: Vec<_> =
        e.0.iter()
            .map(|d| (d.path.to_string(), problem(d).unwrap()))
            .collect();
    assert_eq!(
        got,
        [
            ("Word".to_owned(), Problem::BadTyped),
            ("user".to_owned(), Problem::Expected),
            ("file[\"a b\"]".to_owned(), Problem::BadId),
        ]
        .map(|(p, c)| (p, c))
    );
}

#[test]
fn a_dictionary_checks_references_it_holds() {
    let mut d = dictionary();
    let at = Path::root();
    let known = Ref::parse("word", "hello").unwrap();
    assert_eq!(d.reference(&known, &at).unwrap(), "hello");
    let unknown = Ref::parse("word", "zzz").unwrap();
    assert_eq!(
        d.reference(&unknown, &at),
        Err(Unresolved::Missing(unknown.clone()))
    );
    let other = Ref::parse("file", "zzz").unwrap();
    assert_eq!(d.reference(&other, &at).unwrap(), "zzz");
    assert_eq!(d.expand(&other, &at), Ok(None));
    assert!(d.expand(&unknown, &at).is_err());
    d.declare(substrate_data::KindName::new("file").unwrap());
    assert!(d.reference(&other, &at).is_err());
    assert!(d.get(&known).is_some());
    assert_eq!(
        d.insert(known.kind, known.id.clone(), "Hi".into()),
        Some("Hello".into())
    );
    assert_eq!(d.entries(known.kind).count(), 3);
    assert!(d.holds(known.kind));
}

#[test]
fn expanded_fields_keep_their_wire_form() {
    let n = Notice {
        title: "a".into(),
        subtitle: None,
        tags: vec!["b".into()],
    };
    let back: Notice = read_bytes(&to_bytes(&n)).unwrap();
    assert_eq!(back, n);
    let frame = substrate_data::frame::to_frame(&n);
    assert_eq!(
        substrate_data::frame::read_frame::<Notice>(&frame).unwrap(),
        n
    );
    let described = Notice::shape();
    assert!(matches!(
        described,
        substrate_data::schema::Shape::Struct(_)
    ));
}

/// Checks ids only: `expand` is the default, which keeps references.
struct Checker;

impl Resolve for Checker {
    fn reference(&mut self, r: &Ref, _: &Path) -> Result<RefKey, Unresolved> {
        Ok(r.id.clone())
    }
}

#[test]
fn a_resolver_that_only_checks_keeps_references_unexpanded() {
    let doc = Value::Ref(Ref::parse("word", "hello").unwrap());
    let kept = Checker.expand(&Ref::parse("word", "hello").unwrap(), &Path::root());
    assert_eq!(kept, Ok(None));
    let read = from_value_with::<Value>(&doc, &mut Checker).unwrap();
    assert_eq!(read.value, doc);
}
