//! `#[derive(Schema)]` on a small task-tracker format: reading, problems
//! with their line and path, references, writing back, the binary round
//! trip and the shape.

use substrate_data::schema::Found;
use substrate_data::{Id, Schema, Value, binary, from_text, from_value, text};

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

const TASK: &str = r#"{
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
}"#;

#[test]
fn reads_a_document_and_its_references() {
    let read = from_text::<Task>(TASK).unwrap_or_else(|e| panic!("{e}"));
    let n = read.value;
    assert_eq!(n.name, "@release");
    assert_eq!(n.project, Some(Id::new("apps/notes")));
    assert_eq!(n.color, Some(0xFF8D_6ACC));
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
    let text = r#"{
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
}"#;
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
    let syntax = from_text::<Attachment>("{ file: \"x\",, }")
        .unwrap_err()
        .to_string();
    assert_eq!(syntax, "1:13: expected a key");
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
    // Its typed description, written, keeps the keys tools read.
    let d = Task::shape().describe().write();
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
