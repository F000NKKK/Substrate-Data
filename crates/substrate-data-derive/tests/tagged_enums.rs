//! `#[derive(Schema)]` on tagged enums, newtypes, generics and runtime paths.

use substrate_data::frame::{read_frame, to_frame};
use substrate_data::{Schema, Shape, Value, from_text, read_bytes, read_value, to_bytes};

fn through_every_form<T: Schema + PartialEq + std::fmt::Debug>(x: &T) {
    assert_eq!(&read_value::<T>(&x.write()).expect("from a value"), x);
    assert_eq!(&read_bytes::<T>(&to_bytes(x)).expect("from bytes"), x);
    assert_eq!(read_frame::<T>(&to_frame(x)).as_ref(), Some(x));
    let text = substrate_data::to_text(x);
    assert_eq!(
        &from_text::<T>(&text).expect("from text").value,
        x,
        "{text}"
    );
}

#[derive(Schema, Debug, Clone, PartialEq)]
struct Amount {
    count: u64,
}

/// What a reader of a newer format keeps of a message it doesn't know.
#[derive(Schema, Debug, Clone, PartialEq)]
struct Unknown {
    kind: String,
    #[data(default)]
    count: Option<u64>,
}

#[derive(Schema, Debug, Clone, PartialEq)]
#[data(tag = "kind", rename_all = "camelCase")]
enum Message {
    Ping,
    SendText {
        body: String,
        urgent: Option<bool>,
    },
    Amounted(Amount),
    Counted(u32),
    Absent(Option<u32>),
    #[data(other)]
    Unknown(Unknown),
}

#[test]
fn a_tagged_enum_keeps_its_fields_beside_the_tag() {
    let all = [
        Message::Ping,
        Message::SendText {
            body: "hi".into(),
            urgent: Some(true),
        },
        Message::SendText {
            body: "hi".into(),
            urgent: None,
        },
        Message::Amounted(Amount { count: 3 }),
        Message::Counted(9),
        Message::Absent(None),
        Message::Absent(Some(1)),
    ];
    for m in &all {
        through_every_form(m);
    }
    let v = Message::SendText {
        body: "hi".into(),
        urgent: None,
    }
    .write();
    assert_eq!(v.get("kind"), Some(&Value::from("sendText")));
    assert!(v.get("urgent").is_none());
    assert_eq!(
        Message::Counted(9).write().get("value"),
        Some(&Value::Int(9))
    );
    assert!(Message::Absent(None).write().get("value").is_none());
}

#[test]
fn a_tagged_enum_says_what_is_wrong_with_its_tag() {
    let said = |v: Value| -> String {
        read_value::<Message>(&v)
            .unwrap_err()
            .iter()
            .map(|d| [d.path.to_string(), d.message.clone()].join(": "))
            .collect::<Vec<_>>()
            .join("; ")
    };
    let map = |entries: &[(&str, Value)]| {
        Value::Map(
            entries
                .iter()
                .map(|(k, v)| ((*k).to_owned(), v.clone()))
                .collect(),
        )
    };
    assert_eq!(said(Value::Int(1)), "(root): expected an object, found 1");
    assert_eq!(
        said(map(&[])),
        "(root): missing `kind` (`ping`, `sendText`, `amounted`, `counted`, `absent`)"
    );
    assert_eq!(
        said(map(&[("kind", Value::Int(1))])),
        "kind: expected a name, found 1"
    );
    assert!(
        said(map(&[("kind", "ping".into()), ("extra", Value::Null)]))
            .contains("unknown field `extra`")
    );
    assert!(
        said(map(&[("kind", "counted".into()), ("other", Value::Null)])).contains("unknown field")
    );
    assert!(said(map(&[("kind", "sendText".into())])).contains("missing `body`"));
    assert!(said(map(&[("kind", "amounted".into())])).contains("missing `count`"));
}

#[test]
fn the_other_variant_reads_the_whole_object_for_tags_nobody_else_takes() {
    let v = Value::Map(
        [
            ("kind".to_owned(), Value::from("future")),
            ("count".to_owned(), Value::Int(4)),
        ]
        .into_iter()
        .collect(),
    );
    let read = read_value::<Message>(&v).unwrap();
    assert_eq!(
        read,
        Message::Unknown(Unknown {
            kind: "future".into(),
            count: Some(4)
        })
    );
    let Shape::Enum(e) = Message::shape() else {
        panic!("an enum")
    };
    assert_eq!(e.tag, Some("kind"));
    assert_eq!(e.variants.len(), 5, "`other` is no variant of its own");
}

#[derive(Schema, Debug, Clone, PartialEq)]
struct Nickname(Option<String>);

#[derive(Schema, Debug, Clone, PartialEq)]
struct Pair<T: Schema> {
    first: T,
    second: T,
}

#[test]
fn newtypes_and_generics_follow_their_content() {
    through_every_form(&Nickname(Some("w".into())));
    assert_eq!(Nickname(None).write(), Value::Null);
    assert_eq!(<Nickname as Schema>::absent(), Some(Nickname(None)));
    assert!(Nickname(None).skip());
    assert!(!Nickname(Some(String::new())).skip());
    through_every_form(&Pair {
        first: 1u8,
        second: 2,
    });
    through_every_form(&Pair {
        first: "a".to_owned(),
        second: "b".to_owned(),
    });
}

#[derive(Schema, Debug, Clone, PartialEq)]
#[data(crate = "substrate_data")]
struct Pathed {
    value: u8,
}

#[test]
fn a_container_can_say_where_the_runtime_is() {
    through_every_form(&Pathed { value: 1 });
}
