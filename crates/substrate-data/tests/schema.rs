//! `#[derive(Schema)]` on a small task-tracker format: reading, problems
//! with their line and path, references, writing back, the binary round
//! trip and the shape.

use substrate_data::schema::Found;
use substrate_data::{Id, Ref, Schema, Value, binary, from_text, from_value, text};

substrate_data::kind!(
    User = "user",
    Doc = "doc",
    File = "file",
    Project = "project"
);

/// Where a review stands.
#[derive(Schema, Debug, Clone, PartialEq)]
enum Status {
    Open,
    Closed,
    #[data(rename = "review")]
    Reviewing,
}

/// A review of a task's work.
#[derive(Schema, Debug, Clone, PartialEq)]
struct Review {
    /// Where it stands.
    status: Status,
    #[data(default = "one")]
    hours: f32,
    /// What it looks at: files, or the task's own.
    files: Vec<Id<File>>,
}

fn one() -> f32 {
    1.0
}

#[derive(Schema, Debug, Clone, PartialEq)]
struct Attachment {
    file: Id<File>,
    #[data(default)]
    owner: Option<Id<User>>,
}

/// A part of a task.
#[derive(Schema, Debug, Clone, PartialEq)]
#[data(tag = "type")]
enum Part {
    /// Low, likely and high, in days.
    Estimate {
        range: [f32; 3],
        parent: Option<Id<Doc>>,
    },
    Attachment(Attachment),
    Review(Review),
    Archived,
}

#[derive(Schema, Debug, Clone, PartialEq)]
struct Task {
    name: String,
    project: Option<Id<Project>>,
    #[data(rename = "label")]
    color: Option<u32>,
    parts: Vec<Part>,
    #[data(skip)]
    selected: bool,
    cell: [i32; 2],
}

const TASK: &str = r##"{
  // The release task.
  name: "@@release",
  project: "@project:apps/notes",
  label: 4287457996,
  cell: [0, -2],
  parts: [
    { type: "estimate", range: [0, 1.5, 0], parent: "@doc:roadmap" },
    { type: "attachment", file: "@file:notes.pdf", owner: "@user:alice" },
    { type: "review", status: "review", files: ["@file:diff", "@file:log"], $note: "meta" },
    { type: "archived" },
  ],
}"##;

#[test]
fn reads_a_document_and_its_references() {
    let read = from_text::<Task>(TASK).unwrap_or_else(|e| panic!("{e}"));
    let n = read.value;
    assert_eq!(n.name, "@release");
    assert_eq!(n.project, Some(Id::new("apps/notes")));
    assert_eq!(n.color, Some(0xFF8D6ACC));
    assert_eq!(n.cell, [0, -2]);
    assert_eq!(
        n.parts[0],
        Part::Estimate {
            range: [0.0, 1.5, 0.0],
            parent: Some(Id::new("roadmap"))
        }
    );
    assert_eq!(
        n.parts[1],
        Part::Attachment(Attachment {
            file: Id::new("notes.pdf"),
            owner: Some(Id::new("alice"))
        })
    );
    let Part::Review(b) = &n.parts[2] else {
        panic!()
    };
    assert_eq!((b.hours, &b.status), (1.0, &Status::Reviewing));
    assert_eq!(b.files, [Id::new("diff"), Id::new("log")]);
    assert_eq!(n.parts[3], Part::Archived);

    let refs: Vec<String> = read
        .refs
        .iter()
        .map(|Found { value, path }| format!("{path} {value}"))
        .collect();
    assert_eq!(
        refs,
        [
            "project @project:apps/notes",
            "parts[0].parent @doc:roadmap",
            "parts[1].file @file:notes.pdf",
            "parts[1].owner @user:alice",
            "parts[2].files[0] @file:diff",
            "parts[2].files[1] @file:log",
        ]
    );
}

#[test]
fn every_problem_is_reported_with_its_place() {
    let text = r##"{
  name: 5,
  project: "@file:notes",
  cell: [1],
  lable: 1,
  parts: [
    { type: "estimat", range: [0, 0, 0] },
    { type: "attachment", file: "@user:stone", dat: "y" },
    { type: "review", status: "boil", hours: "heavy", files: ["@file:a b"] },
    { file: "no type" },
  ],
}"##;
    let errors = from_text::<Task>(text).unwrap_err().to_string();
    let expected = [
        "2:9: name: expected a string, found 5",
        "3:12: project: `@file:notes` is a reference to file — this takes `@project:`",
        "4:9: cell: expected 2 items, found 1",
        "5:3: lable: unknown field `lable` — did you mean `label`?",
        "7:13: parts[0].type: unknown `estimat` — did you mean `estimate`?",
        "8:33: parts[1].file: `@user:stone` is a reference to user — this takes `@file:`",
        "8:48: parts[1].dat: unknown field `dat` (expected `file`, `owner`)",
        "9:31: parts[2].status: unknown `boil` (expected `open`, `closed`, `review`)",
        "9:46: parts[2].hours: expected a number, found \"heavy\"",
        "9:63: parts[2].files[0]: `@file:a b`: `a b` isn't an id (not empty, no spaces or control characters)",
        "10:5: parts[3]: missing `type` (`estimate`, `attachment`, `review`, `archived`)",
    ];
    assert_eq!(errors.lines().collect::<Vec<_>>(), expected, "\n{errors}");
}

#[test]
fn missing_fields_and_syntax_errors() {
    let e = from_text::<Attachment>("{ owner: \"@user:x\" }")
        .unwrap_err()
        .to_string();
    assert_eq!(e, "1:1: missing `file`");
    let e = from_text::<Attachment>("{ file: \"x\",, }")
        .unwrap_err()
        .to_string();
    assert_eq!(e, "1:13: expected a key");
}

#[test]
fn writes_back_what_it_read() {
    let task = from_text::<Task>(TASK).unwrap().value;
    let written = task.write();
    let again = from_value::<Task>(&written)
        .unwrap_or_else(|p| panic!("{p:?}"))
        .value;
    assert_eq!(again, task);
    // Through the text form...
    let txt = text::to_string_pretty(&written);
    assert!(
        txt.contains("\"type\": \"attachment\",\n      \"file\": \"@file:notes.pdf\""),
        "{txt}"
    );
    assert!(txt.contains("\"@@release\""), "{txt}");
    assert!(!txt.contains("selected"));
    assert_eq!(from_text::<Task>(&txt).unwrap().value, task);
    // ...and the binary one, which keeps the types.
    let bytes = binary::encode(&written);
    let decoded = binary::decode(&bytes).unwrap();
    assert!(matches!(decoded.get("project"), Some(Value::Ref(_))));
    assert_eq!(from_value::<Task>(&decoded).unwrap().value, task);
}

#[test]
fn shapes_describe_the_format() {
    let d = Task::shape().describe();
    let defs = d.get("defs").unwrap();
    let part = defs.get("Part").unwrap();
    assert_eq!(part.get("tag"), Some(&Value::from("type")));
    let variants = part.get("variants").unwrap().as_list().unwrap();
    assert_eq!(
        variants
            .iter()
            .map(|v| v.get("name").unwrap().as_str().unwrap())
            .collect::<Vec<_>>(),
        ["estimate", "attachment", "review", "archived"]
    );
    assert_eq!(
        variants[0].get("doc"),
        Some(&Value::from("Low, likely and high, in days."))
    );
    // A wrapped struct's fields are the variant's.
    let r_fields = variants[1].get("fields").unwrap().as_list().unwrap();
    assert_eq!(
        r_fields[0].get("shape").unwrap().get("of"),
        Some(&Value::List(vec![Value::from("file")]))
    );
    assert_eq!(r_fields[1].get("required"), Some(&Value::Bool(false)));
    let fields = variants[2].get("fields").unwrap().as_list().unwrap();
    assert_eq!(fields[0].get("doc"), Some(&Value::from("Where it stands.")));
    assert_eq!(fields[1].get("required"), Some(&Value::Bool(false)));
    assert_eq!(
        fields[2]
            .get("shape")
            .unwrap()
            .get("of")
            .unwrap()
            .get("kind"),
        Some(&Value::from("ref"))
    );
}

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
    fn reference(&mut self, r: &Ref, _: &substrate_data::Path) -> Result<String, String> {
        if r.id.ends_with("nope") {
            Err(format!("no {} {}", r.kind, r.id))
        } else {
            Ok(format!("store://{}", r.id))
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
    let v = text::parse_value(r#"{ look: "tree", links: ["@project:nope", "@file:x"] }"#).unwrap();
    let problems: Vec<String> = substrate_data::from_value_with::<Plainly>(&v, &mut Store)
        .unwrap_err()
        .into_iter()
        .map(|p| format!("{}: {}", p.path, p.message))
        .collect();
    assert_eq!(
        problems,
        [
            "look: `tree`: name it by a reference — `@file:tree`",
            "links[0]: no project nope",
            "links[1]: `@file:x` is a reference to file — this takes `@project:`",
        ]
    );
    let shape = Plainly::shape().describe();
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
    let shape = Ranged::shape().describe();
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
    assert_eq!(
        substrate_data::from_text::<Blob>(&text).unwrap().value,
        blob
    );
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
    let from_text = substrate_data::text::parse_value("[2, 1, 2]").unwrap();
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
    let v = substrate_data::Schema::write(&c);
    assert!(v.get("nameProblem").is_some() && v.get("type").is_some());
    assert_eq!(substrate_data::read_value::<Camel>(&v).unwrap(), c);
    let s = CamelVariants::SharedLink {
        link_id: "i".into(),
    };
    let v = substrate_data::Schema::write(&s);
    assert_eq!(v.get("type").and_then(|t| t.as_str()), Some("sharedLink"));
    assert_eq!(v.get("link_id").and_then(|t| t.as_str()), Some("i"));
    assert_eq!(substrate_data::read_value::<CamelVariants>(&v).unwrap(), s);
}

/// A small struct a variant wraps.
#[derive(substrate_data::Schema, Debug, Clone, PartialEq)]
struct Amount {
    count: u64,
}

/// Variants wrapping values that aren't structs, beside one that is.
#[derive(substrate_data::Schema, Debug, Clone, PartialEq)]
#[data(tag = "type")]
enum Reply {
    Accepted(u64),
    Named(String),
    Gave(Option<Amount>),
    Counted(Amount),
    Done,
}

#[test]
fn a_wrapped_value_that_isnt_a_struct_sits_under_value() {
    use substrate_data::schema::WRAPPED;
    let all = [
        Reply::Accepted(7),
        Reply::Named("n".into()),
        Reply::Gave(Some(Amount { count: 2 })),
        Reply::Gave(None),
        Reply::Counted(Amount { count: 3 }),
        Reply::Done,
    ];
    for r in &all {
        let v = r.write();
        assert_eq!(
            substrate_data::read_value::<Reply>(&v).unwrap(),
            *r,
            "{v:?}"
        );
        let bytes = substrate_data::schema::to_bytes(r);
        assert_eq!(substrate_data::read_bytes::<Reply>(&bytes).unwrap(), *r);
    }
    let accepted = Reply::Accepted(7).write();
    assert_eq!(accepted.get(WRAPPED), Some(&Value::Int(7)));
    // A struct's fields stay beside the tag; a missing Option is left out.
    let counted = Reply::Counted(Amount { count: 3 }).write();
    assert!(counted.get(WRAPPED).is_none() && counted.get("count").is_some());
    assert!(Reply::Gave(None).write().get(WRAPPED).is_none());
    // The shape says where the value is.
    let substrate_data::Shape::Enum(e) = Reply::shape() else {
        panic!("an enum")
    };
    let named = e.variants.iter().find(|v| v.name == "named").unwrap();
    assert_eq!(named.fields[0].name, WRAPPED);
    assert!(named.fields[0].required);
}

/// A type of a format's own, stored as three floats (red, green, blue).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Color([f32; 3]);

impl Schema for Color {
    fn shape() -> substrate_data::Shape {
        substrate_data::Shape::Named {
            name: "color",
            of: <[f32; 3]>::shape,
        }
    }
    fn read(v: &substrate_data::ValueRef<'_>, cx: &mut substrate_data::Reader) -> Option<Self> {
        <[f32; 3]>::read(v, cx).map(Color)
    }
    fn write(&self) -> Value {
        self.0.write()
    }
    fn pack(&self, p: &mut substrate_data::frame::Packer) {
        self.0.pack(p);
    }
    fn unpack(u: &mut substrate_data::frame::Unpacker<'_>) -> Option<Self> {
        <[f32; 3]>::unpack(u).map(Color)
    }
}

#[derive(Schema, Debug, Clone, PartialEq)]
struct Label {
    text: String,
    color: Color,
}

#[test]
fn named_shapes_and_at_signs_survive_every_form() {
    let m = Label {
        text: "@everyone".into(),
        color: Color([1.0, 0.5, 0.25]),
    };
    let txt = substrate_data::to_text(&m);
    assert!(txt.contains("\"@@everyone\""), "{txt}");
    assert_eq!(from_text::<Label>(&txt).unwrap().value, m);
    let bytes = substrate_data::to_bytes(&m);
    assert_eq!(substrate_data::read_bytes::<Label>(&bytes).unwrap(), m);
    let frame = substrate_data::frame::to_frame(&m);
    assert_eq!(
        substrate_data::frame::read_frame::<Label>(&frame),
        Some(m.clone())
    );
    // A tool with no code for it reads the frame by its shape...
    let seen = substrate_data::frame::read_shaped(&frame, &Label::shape()).unwrap();
    assert_eq!(substrate_data::read_value::<Label>(&seen).unwrap(), m);
    // ...and sees the name it may have an editor for.
    let d = Label::shape().describe();
    let color = &d
        .get("defs")
        .unwrap()
        .get("Label")
        .unwrap()
        .get("fields")
        .unwrap()
        .as_list()
        .unwrap()[1];
    let shape = color.get("shape").unwrap();
    assert_eq!(shape.get("kind"), Some(&Value::from("named")));
    assert_eq!(shape.get("name"), Some(&Value::from("color")));
}

#[test]
fn an_atom_starting_with_at_round_trips_in_binary() {
    let a = substrate_data::Atom::new("@everyone");
    let bytes = substrate_data::to_bytes(&a);
    assert_eq!(
        substrate_data::read_bytes::<substrate_data::Atom>(&bytes).unwrap(),
        a
    );
    assert_eq!(
        substrate_data::binary::decode(&bytes).unwrap(),
        Value::from("@@everyone")
    );
}

#[derive(Schema, Debug, Clone, PartialEq)]
struct Lists {
    set: std::collections::BTreeSet<u32>,
    pair: (f32, String),
    names: substrate_data::OneOrList<String>,
    path: substrate_data::Path,
}

#[test]
fn list_shaped_types_pack_as_lists() {
    let l = Lists {
        set: [3, 1, 2].into_iter().collect(),
        pair: (0.5, "@at".into()),
        names: substrate_data::OneOrList(vec!["one".into()]),
        path: substrate_data::Path::root().key("a").index(2),
    };
    let frame = substrate_data::frame::to_frame(&l);
    assert_eq!(
        substrate_data::frame::read_frame::<Lists>(&frame),
        Some(l.clone())
    );
    // A tool with no code for it reads the same frame by its shape alone.
    let seen = substrate_data::frame::read_shaped(&frame, &Lists::shape()).unwrap();
    assert_eq!(substrate_data::read_value::<Lists>(&seen).unwrap(), l);
}

#[derive(Schema, Debug, Clone, PartialEq)]
struct Palette {
    colors: Vec<Color>,
    accent: Option<Color>,
}

#[test]
fn named_shapes_read_by_shape_inside_lists_and_options() {
    let p = Palette {
        colors: vec![Color([0.0, 1.0, 0.5]), Color([0.125, 0.25, 0.75])],
        accent: Some(Color([1.0, 0.5, 0.0])),
    };
    let frame = substrate_data::frame::to_frame(&p);
    let seen = substrate_data::frame::read_shaped(&frame, &Palette::shape()).unwrap();
    assert_eq!(substrate_data::read_value::<Palette>(&seen).unwrap(), p);
}
