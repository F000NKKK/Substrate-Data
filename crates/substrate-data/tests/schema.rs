//! `#[derive(Schema)]` on a small scene-like format: reading, problems with
//! their line and path, references, writing back, the binary round trip and
//! the shape.

use substrate_data::schema::Found;
use substrate_data::{Id, Ref, Schema, Value, binary, from_text, from_value, text};

substrate_data::kind!(
    Mesh = "mesh",
    Material = "material",
    Entity = "entity",
    Prefab = "prefab"
);

/// How a body moves.
#[derive(Schema, Debug, Clone, PartialEq)]
enum Motion {
    Static,
    Dynamic,
    #[data(rename = "kinematic")]
    Scripted,
}

/// A rigid body.
#[derive(Schema, Debug, Clone, PartialEq)]
struct Body {
    /// How it moves.
    motion: Motion,
    #[data(default = "one")]
    mass: f32,
    /// What it collides as: a mesh, or the entity's own.
    colliders: Vec<Id<Mesh>>,
}

fn one() -> f32 {
    1.0
}

#[derive(Schema, Debug, Clone, PartialEq)]
struct Renderable {
    mesh: Id<Mesh>,
    #[data(default)]
    material: Option<Id<Material>>,
}

/// A component.
#[derive(Schema, Debug, Clone, PartialEq)]
#[data(tag = "type")]
enum Component {
    /// Where it is.
    Transform {
        position: [f32; 3],
        parent: Option<Id<Entity>>,
    },
    Renderable(Renderable),
    Body(Body),
    Hidden,
}

#[derive(Schema, Debug, Clone, PartialEq)]
struct Node {
    name: String,
    prefab: Option<Id<Prefab>>,
    #[data(rename = "tint")]
    color: Option<u32>,
    components: Vec<Component>,
    #[data(skip)]
    selected: bool,
    cell: [i32; 2],
}

const SCENE: &str = r##"{
  // The player.
  name: "@@player",
  prefab: "@prefab:actors/hero",
  tint: 4287457996,
  cell: [0, -2],
  components: [
    { type: "transform", position: [0, 1.5, 0], parent: "@entity:world" },
    { type: "renderable", mesh: "@mesh:hero.mesh", material: "@material:skin" },
    { type: "body", motion: "kinematic", colliders: ["@mesh:capsule", "@mesh:feet"], $note: "meta" },
    { type: "hidden" },
  ],
}"##;

#[test]
fn reads_a_document_and_its_references() {
    let read = from_text::<Node>(SCENE).unwrap_or_else(|e| panic!("{e}"));
    let n = read.value;
    assert_eq!(n.name, "@player");
    assert_eq!(n.prefab, Some(Id::new("actors/hero")));
    assert_eq!(n.color, Some(0xFF8D6ACC));
    assert_eq!(n.cell, [0, -2]);
    assert_eq!(
        n.components[0],
        Component::Transform {
            position: [0.0, 1.5, 0.0],
            parent: Some(Id::new("world"))
        }
    );
    assert_eq!(
        n.components[1],
        Component::Renderable(Renderable {
            mesh: Id::new("hero.mesh"),
            material: Some(Id::new("skin"))
        })
    );
    let Component::Body(b) = &n.components[2] else {
        panic!()
    };
    assert_eq!((b.mass, &b.motion), (1.0, &Motion::Scripted));
    assert_eq!(b.colliders, [Id::new("capsule"), Id::new("feet")]);
    assert_eq!(n.components[3], Component::Hidden);

    let refs: Vec<String> = read
        .refs
        .iter()
        .map(|Found { value, path }| format!("{path} {value}"))
        .collect();
    assert_eq!(
        refs,
        [
            "prefab @prefab:actors/hero",
            "components[0].parent @entity:world",
            "components[1].mesh @mesh:hero.mesh",
            "components[1].material @material:skin",
            "components[2].colliders[0] @mesh:capsule",
            "components[2].colliders[1] @mesh:feet",
        ]
    );
}

#[test]
fn every_problem_is_reported_with_its_place() {
    let text = r##"{
  name: 5,
  prefab: "@mesh:hero",
  cell: [1],
  tnt: 1,
  components: [
    { type: "transfrom", position: [0, 0, 0] },
    { type: "renderable", mesh: "@material:stone", mat: "y" },
    { type: "body", motion: "boil", mass: "heavy", colliders: ["@mesh:a b"] },
    { mesh: "no type" },
  ],
}"##;
    let errors = from_text::<Node>(text).unwrap_err().to_string();
    let expected = [
        "2:9: name: expected a string, found 5",
        "3:11: prefab: `@mesh:hero` is a reference to mesh — this takes `@prefab:`",
        "4:9: cell: expected 2 items, found 1",
        "5:3: tnt: unknown field `tnt` — did you mean `tint`?",
        "7:13: components[0].type: unknown `transfrom` — did you mean `transform`?",
        "8:33: components[1].mesh: `@material:stone` is a reference to material — this takes `@mesh:`",
        "8:52: components[1].mat: unknown field `mat` (expected `mesh`, `material`)",
        "9:29: components[2].motion: unknown `boil` (expected `static`, `dynamic`, `kinematic`)",
        "9:43: components[2].mass: expected a number, found \"heavy\"",
        "9:64: components[2].colliders[0]: `@mesh:a b`: `a b` isn't an id (not empty, no spaces or control characters)",
        "10:5: components[3]: missing `type` (`transform`, `renderable`, `body`, `hidden`)",
    ];
    assert_eq!(errors.lines().collect::<Vec<_>>(), expected, "\n{errors}");
}

#[test]
fn missing_fields_and_syntax_errors() {
    let e = from_text::<Renderable>("{ material: \"@material:x\" }")
        .unwrap_err()
        .to_string();
    assert_eq!(e, "1:1: missing `mesh`");
    let e = from_text::<Renderable>("{ mesh: \"x\",, }")
        .unwrap_err()
        .to_string();
    assert_eq!(e, "1:13: expected a key");
}

#[test]
fn writes_back_what_it_read() {
    let node = from_text::<Node>(SCENE).unwrap().value;
    let written = node.write();
    let again = from_value::<Node>(&written)
        .unwrap_or_else(|p| panic!("{p:?}"))
        .value;
    assert_eq!(again, node);
    // Through the text form...
    let txt = text::to_string_pretty(&written);
    assert!(
        txt.contains("\"type\": \"renderable\",\n      \"mesh\": \"@mesh:hero.mesh\""),
        "{txt}"
    );
    assert!(txt.contains("\"@@player\""), "{txt}");
    assert!(!txt.contains("selected"));
    assert_eq!(from_text::<Node>(&txt).unwrap().value, node);
    // ...and the binary one, which keeps the types.
    let bytes = binary::encode(&written);
    let decoded = binary::decode(&bytes).unwrap();
    assert!(matches!(decoded.get("prefab"), Some(Value::Ref(_))));
    assert_eq!(from_value::<Node>(&decoded).unwrap().value, node);
}

#[test]
fn shapes_describe_the_format() {
    let d = Node::shape().describe();
    let defs = d.get("defs").unwrap();
    let comp = defs.get("Component").unwrap();
    assert_eq!(comp.get("tag"), Some(&Value::from("type")));
    let variants = comp.get("variants").unwrap().as_list().unwrap();
    assert_eq!(
        variants
            .iter()
            .map(|v| v.get("name").unwrap().as_str().unwrap())
            .collect::<Vec<_>>(),
        ["transform", "renderable", "body", "hidden"]
    );
    assert_eq!(variants[0].get("doc"), Some(&Value::from("Where it is.")));
    // A wrapped struct's fields are the variant's.
    let r_fields = variants[1].get("fields").unwrap().as_list().unwrap();
    assert_eq!(
        r_fields[0].get("shape").unwrap().get("of"),
        Some(&Value::List(vec![Value::from("mesh")]))
    );
    assert_eq!(r_fields[1].get("required"), Some(&Value::Bool(false)));
    let fields = variants[2].get("fields").unwrap().as_list().unwrap();
    assert_eq!(fields[0].get("doc"), Some(&Value::from("How it moves.")));
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
    #[data(with = "substrate_data::with::RefTo<(Mesh, Prefab)>")]
    look: String,
    #[data(with = "substrate_data::with::Opt<substrate_data::with::RefTo<Material>>")]
    material: Option<String>,
    #[data(
        with = "substrate_data::with::List<substrate_data::with::RefTo<Entity>>",
        default
    )]
    links: Vec<String>,
}

/// Resolves references against what there is: `…nope` isn't.
struct Assets;

impl substrate_data::Resolve for Assets {
    fn reference(&mut self, r: &Ref, _: &substrate_data::Path) -> Result<String, String> {
        if r.id.ends_with("nope") {
            Err(format!("no {} {}", r.kind, r.id))
        } else {
            Ok(format!("res://{}", r.id))
        }
    }
}

#[test]
fn codecs_keep_plain_types_and_resolve() {
    let v = text::parse_value(
        r#"{ look: "@prefab:tree", material: "@material:bark", links: ["@entity:root"] }"#,
    )
    .unwrap();
    let read = substrate_data::from_value_with::<Plainly>(&v, &mut Assets).unwrap();
    assert_eq!(
        read.value,
        Plainly {
            look: "res://tree".into(),
            material: Some("res://bark".into()),
            links: vec!["res://root".into()]
        }
    );
    let v = text::parse_value(r#"{ look: "tree", links: ["@entity:nope", "@mesh:x"] }"#).unwrap();
    let problems: Vec<String> = substrate_data::from_value_with::<Plainly>(&v, &mut Assets)
        .unwrap_err()
        .into_iter()
        .map(|p| format!("{}: {}", p.path, p.message))
        .collect();
    assert_eq!(
        problems,
        [
            "look: `tree`: name it by a reference — `@mesh:tree`",
            "links[0]: no entity nope",
            "links[1]: `@mesh:x` is a reference to mesh — this takes `@entity:`",
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
        Some(&Value::List(vec![
            Value::from("mesh"),
            Value::from("prefab")
        ]))
    );
    assert_eq!(fields[2].get("required"), Some(&Value::Bool(false)));
}

#[derive(Schema, Debug, PartialEq)]
struct Ranged {
    #[data(range = "1..=99")]
    stack: u32,
    #[data(range = "-16..=32")]
    from: Option<[f64; 3]>,
}

#[test]
fn ranges_are_checked_and_described() {
    assert_eq!(
        from_text::<Ranged>("{ stack: 64, from: [0, 8, 32] }")
            .unwrap()
            .value,
        Ranged {
            stack: 64,
            from: Some([0.0, 8.0, 32.0])
        }
    );
    let e = from_text::<Ranged>("{ stack: 100, from: [0, 40.5, 0] }")
        .unwrap_err()
        .to_string();
    assert_eq!(
        e.lines().collect::<Vec<_>>(),
        [
            "1:10: stack: 100 is out of range (1..=99)",
            "1:21: from: 40.5 is out of range (-16..=32)"
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
    let path = std::path::PathBuf::from("/home/a/.cache/assets/21");
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
    ShapedRecipe { item_id: String },
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
    let s = CamelVariants::ShapedRecipe {
        item_id: "i".into(),
    };
    let v = substrate_data::Schema::write(&s);
    assert_eq!(v.get("type").and_then(|t| t.as_str()), Some("shapedRecipe"));
    assert_eq!(v.get("item_id").and_then(|t| t.as_str()), Some("i"));
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

/// A domain type of a format's own, stored as three floats.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Vec3([f32; 3]);

impl Schema for Vec3 {
    fn shape() -> substrate_data::Shape {
        substrate_data::Shape::Named {
            name: "vec3",
            of: <[f32; 3]>::shape,
        }
    }
    fn read(v: &substrate_data::ValueRef<'_>, cx: &mut substrate_data::Reader) -> Option<Self> {
        <[f32; 3]>::read(v, cx).map(Vec3)
    }
    fn write(&self) -> Value {
        self.0.write()
    }
    fn pack(&self, p: &mut substrate_data::frame::Packer) {
        self.0.pack(p);
    }
    fn unpack(u: &mut substrate_data::frame::Unpacker<'_>) -> Option<Self> {
        <[f32; 3]>::unpack(u).map(Vec3)
    }
}

#[derive(Schema, Debug, Clone, PartialEq)]
struct Marker {
    label: String,
    at: Vec3,
}

#[test]
fn named_shapes_and_at_signs_survive_every_form() {
    let m = Marker {
        label: "@spawn".into(),
        at: Vec3([1.0, 2.5, -3.0]),
    };
    let txt = substrate_data::to_text(&m);
    assert!(txt.contains("\"@@spawn\""), "{txt}");
    assert_eq!(from_text::<Marker>(&txt).unwrap().value, m);
    let bytes = substrate_data::to_bytes(&m);
    assert_eq!(substrate_data::read_bytes::<Marker>(&bytes).unwrap(), m);
    let frame = substrate_data::frame::to_frame(&m);
    assert_eq!(
        substrate_data::frame::read_frame::<Marker>(&frame),
        Some(m.clone())
    );
    // A tool with no code for it reads the frame by its shape...
    let seen = substrate_data::frame::read_shaped(&frame, &Marker::shape()).unwrap();
    assert_eq!(substrate_data::read_value::<Marker>(&seen).unwrap(), m);
    // ...and sees the name it may have an editor for.
    let d = Marker::shape().describe();
    let at = &d
        .get("defs")
        .unwrap()
        .get("Marker")
        .unwrap()
        .get("fields")
        .unwrap()
        .as_list()
        .unwrap()[1];
    let shape = at.get("shape").unwrap();
    assert_eq!(shape.get("kind"), Some(&Value::from("named")));
    assert_eq!(shape.get("name"), Some(&Value::from("vec3")));
}

#[test]
fn an_atom_starting_with_at_round_trips_in_binary() {
    let a = substrate_data::Atom::new("@spawn");
    let bytes = substrate_data::to_bytes(&a);
    assert_eq!(
        substrate_data::read_bytes::<substrate_data::Atom>(&bytes).unwrap(),
        a
    );
    assert_eq!(
        substrate_data::binary::decode(&bytes).unwrap(),
        Value::from("@@spawn")
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
struct Path3 {
    points: Vec<Vec3>,
    end: Option<Vec3>,
}

#[test]
fn named_shapes_read_by_shape_inside_lists_and_options() {
    let p = Path3 {
        points: vec![Vec3([0.0, 1.0, 2.0]), Vec3([-1.5, 0.25, 8.0])],
        end: Some(Vec3([9.0, 9.5, 10.0])),
    };
    let frame = substrate_data::frame::to_frame(&p);
    let seen = substrate_data::frame::read_shaped(&frame, &Path3::shape()).unwrap();
    assert_eq!(substrate_data::read_value::<Path3>(&seen).unwrap(), p);
}
