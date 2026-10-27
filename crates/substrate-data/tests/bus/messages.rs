//! Messages written and read straight, as the value would be.

use substrate_data::{Schema, Value, binary};

/// How a result is laid out (a unit enum: a frame packs it by index).
#[derive(Schema, Debug, Clone, PartialEq)]
pub(crate) enum Mode {
    Draft,
    Final,
    #[data(rename = "arch")]
    Archived,
}

/// What a bus carries: written and read straight, without a value tree.
#[derive(Schema, Debug, Clone, PartialEq)]
pub(crate) struct Message {
    pub(crate) name: String,
    pub(crate) count: u32,
    pub(crate) ratio: f32,
    pub(crate) tags: Vec<String>,
    pub(crate) mode: Mode,
    pub(crate) note: Option<String>,
    #[data(default)]
    pub(crate) extra: i64,
    #[data(with = "substrate_data::with::Raw")]
    pub(crate) bytes: Vec<u8>,
}

#[test]
fn messages_go_straight_and_as_the_value_would() {
    let m = Message {
        name: "report".into(),
        count: 3,
        ratio: 0.5,
        tags: vec!["a".into(), "b".into(), "a".into()],
        mode: Mode::Archived,
        note: None,
        extra: -7,
        bytes: vec![1, 2, 3],
    };
    let straight = substrate_data::to_bytes(&m);
    // The same value either way.
    let via_value = binary::encode(&m.write());
    assert_eq!(
        binary::decode(&straight).unwrap(),
        binary::decode(&via_value).unwrap()
    );
    let mut d = binary::Decoder::open(&straight).unwrap();
    assert_eq!(
        Message::decode(&mut d).as_ref(),
        Some(&m),
        "read straight, not the slow way"
    );
    assert!(d.at_end());
    assert_eq!(substrate_data::read_bytes::<Message>(&straight).unwrap(), m);
    // A field left out takes its default; an unknown one sends it the slow way, which says so.
    let mut fewer = m.write();
    if let Value::Map(map) = &mut fewer {
        map.remove("extra");
        map.remove("note");
    }
    let fewer = binary::encode(&fewer);
    let mut older = binary::Decoder::open(&fewer).unwrap();
    assert_eq!(Message::decode(&mut older).map(|r| r.extra), Some(0));
    let mut more = m.write();
    if let Value::Map(map) = &mut more {
        map.insert("nope", Value::Int(1));
    }
    let bytes = binary::encode(&more);
    assert!(Message::decode(&mut binary::Decoder::open(&bytes).unwrap()).is_none());
    assert!(
        substrate_data::read_bytes::<Message>(&bytes)
            .unwrap_err()
            .iter()
            .any(|p| p.message.contains("nope"))
    );
}
