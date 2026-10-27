//! Typed reference ids: a kind's id type read from text, checked with its
//! path, packed natively in a frame, and described in the shape.

use substrate_data::frame::{read_frame, read_shaped, to_frame};
use substrate_data::{
    Code, Diagnostic, Id, KindName, Problem, Schema, ShapeDoc, Uid, Value, from_text, read_value,
};

substrate_data::kind!(User = "user": u64, Doc = "doc", Upload = "upload": Uid);

#[derive(Schema, Debug, Clone, PartialEq)]
struct Share {
    owner: Id<User>,
    doc: Id<Doc>,
    upload: Option<Id<Upload>>,
}

fn share() -> Share {
    Share {
        owner: Id::new(42u64),
        doc: Id::new("notes"),
        upload: Some(Id::new(Uid(0x00ff_u128 << 64 | 7))),
    }
}

#[test]
fn ids_read_as_their_kinds_id_type() {
    let read = from_text::<Share>(
        r#"{ owner: "@user:42", doc: "@doc:notes", upload: "@upload:00000000000000ff0000000000000007" }"#,
    )
    .unwrap()
    .value;
    assert_eq!(read, share());
    assert_eq!(read.owner.id, 42);
}

#[test]
fn an_id_of_the_wrong_type_is_a_typed_problem_with_its_path() {
    let problems: Vec<Diagnostic> = from_text::<Share>(r#"{ owner: "@user:abc", doc: "@doc:x" }"#)
        .unwrap_err()
        .0;
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].code, Some(Code::Data(Problem::BadId)));
    assert_eq!(problems[0].path.to_string(), "owner");
    assert_eq!(
        problems[0].message,
        "`@user:abc`: `abc` isn't a user id (expected an unsigned integer)"
    );
    assert_eq!(Problem::BadId.code(), "SD0010");
}

#[test]
fn frames_pack_ids_natively_and_read_back_by_shape() {
    let s = share();
    let frame = to_frame(&s);
    assert_eq!(read_frame::<Share>(&frame), Some(s.clone()));
    // The user id is one varint, not `user` and `42` as text.
    let without = to_frame(&Share {
        upload: None,
        ..s.clone()
    });
    assert!(without.len() < 16, "{without:?}");
    let seen = read_shaped(&frame, &Share::shape()).unwrap();
    assert_eq!(read_value::<Share>(&seen).unwrap(), s);
}

#[test]
fn shapes_describe_kinds_and_id_types() {
    let doc = ShapeDoc::of(&Share::shape());
    let written = doc.write();
    let fields = written.get("defs").and_then(|d| d.get("Share")).unwrap();
    let owner = &fields.get("fields").unwrap().as_list().unwrap()[0];
    let shape = owner.get("shape").unwrap();
    assert_eq!(shape.get("kind"), Some(&Value::from("ref")));
    assert_eq!(
        shape.get("of"),
        Some(&Value::List(vec![Value::from("user")]))
    );
    assert_eq!(
        shape.get("id").and_then(|i| i.get("kind")),
        Some(&Value::from("int"))
    );
    // The typed description reads back from what it wrote.
    assert_eq!(read_value::<ShapeDoc>(&written).unwrap(), doc);
}

#[test]
fn kind_names_are_checked() {
    assert!(KindName::new("user").is_ok());
    assert!(KindName::new("User").is_err());
    assert!(KindName::new("").is_err());
    assert_eq!(KindName::new("doc").unwrap(), "doc");
}
