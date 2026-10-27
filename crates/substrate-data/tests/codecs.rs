//! Fields read through codecs (references resolved, raw bytes, paths and
//! sets), numeric ranges, and `rename_all`.

use substrate_data::{Ref, Schema, Value, from_text, text};

substrate_data::kind!(
    User = "user",
    Doc = "doc",
    File = "file",
    Project = "project"
);

/// A plain-string model read through codecs, references resolved.
#[derive(Schema, Debug, PartialEq)]
struct Plainly {
    #[data(with = "substrate_data::with::RefTo<(File, Doc)>")]
    look: String,
    #[data(with = "substrate_data::with::Opt<substrate_data::with::RefTo<User>>")]
    owner: Option<String>,
    #[data(
        with = "substrate_data::with::List<substrate_data::with::RefTo<Project>>",
        default
    )]
    links: Vec<String>,
}

/// Resolves references against what there is: `…nope` isn't.
struct Store;

impl substrate_data::Resolve for Store {
    fn reference(
        &mut self,
        r: &Ref,
        _: &substrate_data::Path,
    ) -> Result<substrate_data::RefKey, substrate_data::Unresolved> {
        if r.id.as_str().ends_with("nope") {
            Err(substrate_data::Unresolved::Missing(r.clone()))
        } else {
            substrate_data::RefKey::new(&format!("store://{}", r.id)).map_err(|e| {
                substrate_data::Unresolved::Refused {
                    reference: r.clone(),
                    reason: e.to_string(),
                }
            })
        }
    }
}

#[test]
fn codecs_keep_plain_types_and_resolve() {
    let v =
        text::parse_value(r#"{ look: "@doc:tree", owner: "@user:bob", links: ["@project:root"] }"#)
            .unwrap();
    let read = substrate_data::from_value_with::<Plainly>(&v, &mut Store).unwrap();
    assert_eq!(
        read.value,
        Plainly {
            look: "store://tree".into(),
            owner: Some("store://bob".into()),
            links: vec!["store://root".into()]
        }
    );
    let wrong =
        text::parse_value(r#"{ look: "tree", links: ["@project:nope", "@file:x"] }"#).unwrap();
    let problems: Vec<String> = substrate_data::from_value_with::<Plainly>(&wrong, &mut Store)
        .unwrap_err()
        .into_iter()
        .map(|p| format!("{}: {}", p.path, p.message))
        .collect();
    assert_eq!(
        problems,
        [
            "look: `tree`: name it by a reference — `@file:tree`",
            "links[0]: `@project:nope`: there is no such project",
            "links[1]: `@file:x` is a reference to file — this takes `@project:`",
        ]
    );
    let shape = Plainly::shape().describe().write();
    let fields = shape
        .get("defs")
        .unwrap()
        .get("Plainly")
        .unwrap()
        .get("fields")
        .unwrap()
        .as_list()
        .unwrap();
    assert_eq!(
        fields[0].get("shape").unwrap().get("of"),
        Some(&Value::List(vec![Value::from("file"), Value::from("doc")]))
    );
    assert_eq!(fields[2].get("required"), Some(&Value::Bool(false)));
}

#[derive(Schema, Debug, PartialEq)]
struct Ranged {
    #[data(range = "1..=99")]
    copies: u32,
    #[data(range = "-16..=32")]
    from: Option<[f64; 3]>,
}

#[test]
fn ranges_are_checked_and_described() {
    assert_eq!(
        from_text::<Ranged>("{ copies: 64, from: [0, 8, 32] }")
            .unwrap()
            .value,
        Ranged {
            copies: 64,
            from: Some([0.0, 8.0, 32.0])
        }
    );
    let e = from_text::<Ranged>("{ copies: 100, from: [0, 40.5, 0] }")
        .unwrap_err()
        .to_string();
    assert_eq!(
        e.lines().collect::<Vec<_>>(),
        [
            "1:11: copies: 100 is out of range (1..=99)",
            "1:22: from: 40.5 is out of range (-16..=32)"
        ]
    );
    let shape = Ranged::shape().describe().write();
    let fields = shape
        .get("defs")
        .unwrap()
        .get("Ranged")
        .unwrap()
        .get("fields")
        .unwrap()
        .as_list()
        .unwrap();
    assert_eq!(
        fields[0].get("range"),
        Some(&Value::List(vec![Value::Float(1.0), Value::Float(99.0)]))
    );
}

#[derive(Schema, Debug, PartialEq)]
struct Blob {
    #[data(with = "substrate_data::with::Raw")]
    bytes: Vec<u8>,
}

#[test]
fn raw_bytes_round_trip_through_text() {
    let blob = Blob {
        bytes: vec![0, 1, 2, 255],
    };
    let text = substrate_data::to_text(&blob);
    assert!(text.contains("@bytes:"), "{text}");
    assert_eq!(from_text::<Blob>(&text).unwrap().value, blob);
}

#[test]
fn paths_and_sets_round_trip() {
    let path = std::path::PathBuf::from("/home/a/.cache/files/21");
    assert_eq!(
        substrate_data::read_bytes::<std::path::PathBuf>(&substrate_data::to_bytes(&path)).unwrap(),
        path
    );
    let words: std::collections::BTreeSet<String> = ["b".into(), "a".into()].into();
    assert_eq!(
        substrate_data::read_bytes::<std::collections::BTreeSet<String>>(
            &substrate_data::to_bytes(&words)
        )
        .unwrap(),
        words
    );
    let numbers: std::collections::BTreeSet<u32> = [3, 1, 2].into();
    assert_eq!(
        substrate_data::read_bytes::<std::collections::BTreeSet<u32>>(&substrate_data::to_bytes(
            &numbers
        ))
        .unwrap(),
        numbers
    );
    let from_text = text::parse_value("[2, 1, 2]").unwrap();
    assert_eq!(
        substrate_data::read_value::<std::collections::BTreeSet<u32>>(&from_text).unwrap(),
        [1, 2].into()
    );
}

#[derive(substrate_data::Schema, Debug, PartialEq)]
#[data(rename_all = "camelCase")]
struct Camel {
    name_problem: Option<String>,
    r#type: String,
}

#[derive(substrate_data::Schema, Debug, PartialEq)]
#[data(rename_all = "camelCase", tag = "type")]
enum CamelVariants {
    SharedLink { link_id: String },
    Plain,
}

#[test]
fn rename_all_camel_case_names_fields_and_variants() {
    let c = Camel {
        name_problem: Some("x".into()),
        r#type: "t".into(),
    };
    let v = Schema::write(&c);
    assert!(v.get("nameProblem").is_some() && v.get("type").is_some());
    assert_eq!(substrate_data::read_value::<Camel>(&v).unwrap(), c);
    let s = CamelVariants::SharedLink {
        link_id: "i".into(),
    };
    let written = Schema::write(&s);
    assert_eq!(
        written.get("type").and_then(|t| t.as_str()),
        Some("sharedLink")
    );
    assert_eq!(written.get("link_id").and_then(|t| t.as_str()), Some("i"));
    assert_eq!(
        substrate_data::read_value::<CamelVariants>(&written).unwrap(),
        s
    );
}
