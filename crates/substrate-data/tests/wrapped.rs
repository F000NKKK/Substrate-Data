//! Tagged variants wrapping values, named types, atoms and list-shaped
//! types, in every form.

use substrate_data::{Schema, Value, binary, from_text};

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

impl substrate_data::NamedType for Color {
    const NAME: &'static str = "color";
    type Stored = [f32; 3];
    fn to_stored(&self) -> [f32; 3] {
        self.0
    }
    fn from_stored(rgb: [f32; 3]) -> Option<Self> {
        rgb.iter()
            .all(|c| (0.0..=1.0).contains(c))
            .then_some(Color(rgb))
    }
}

substrate_data::named_schema!(Color);

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
    let d = Label::shape().describe().write();
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
    assert_eq!(binary::decode(&bytes).unwrap(), Value::from("@@everyone"));
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
