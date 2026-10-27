//! [`ShapeNode`]: one shape as data, written `{ kind: "…", … }`.

use super::DefinitionDoc;
use super::definitions::Definitions;
use crate::model::value::KindName;
use crate::schema::Shape;

/// One shape as data, tagged by `kind`; a struct or enum is a `def`, its
/// definition written once in the description's `defs`.
#[derive(Debug, Clone, PartialEq, crate::Schema)]
#[data(tag = "kind")]
pub enum ShapeNode {
    /// Any value.
    Any,
    /// `true` or `false`.
    Bool,
    /// An integer in `min..=max`.
    Int {
        /// The least it may be.
        min: i64,
        /// The most it may be.
        max: i64,
    },
    /// A number with a fraction, `bits` wide.
    Float {
        /// 32 or 64.
        bits: u32,
    },
    /// Plain text.
    String,
    /// A name that repeats.
    Atom,
    /// Bytes as they are.
    Bytes,
    /// A reference of one of `of` (any kind if empty), its id of shape `id`.
    Ref {
        /// The kinds it takes.
        of: Vec<KindName>,
        /// The shape of its id.
        id: Box<ShapeNode>,
    },
    /// A type tools may know by `name`, stored as `of`.
    Named {
        /// What tools know it by.
        name: String,
        /// What it's stored as.
        of: Box<ShapeNode>,
    },
    /// A list of `of`.
    List {
        /// Each item's shape.
        of: Box<ShapeNode>,
    },
    /// An object of any keys, every value `of`.
    Map {
        /// Each value's shape.
        of: Box<ShapeNode>,
    },
    /// `of` or null.
    Optional {
        /// The shape when it's there.
        of: Box<ShapeNode>,
    },
    /// The struct or enum `name`, defined in `defs`.
    Def {
        /// Its name in `defs`.
        name: String,
    },
}

impl ShapeNode {
    /// `shape` as data, its structs and enums defined in `defs`.
    pub(super) fn of(shape: &Shape, defs: &mut Definitions) -> ShapeNode {
        let mut inner = |s: Shape| Box::new(ShapeNode::of(&s, defs));
        match shape {
            Shape::Any => ShapeNode::Any,
            Shape::Bool => ShapeNode::Bool,
            Shape::Int { min, max } => ShapeNode::Int {
                min: *min,
                max: *max,
            },
            Shape::Float { bits } => ShapeNode::Float { bits: *bits },
            Shape::Str => ShapeNode::String,
            Shape::Atom => ShapeNode::Atom,
            Shape::Bytes => ShapeNode::Bytes,
            Shape::Ref { kinds, id } => ShapeNode::Ref {
                of: kinds.iter().filter_map(|k| KindName::new(k).ok()).collect(),
                id: inner(id()),
            },
            Shape::Named(n) => ShapeNode::Named {
                name: n.name().to_owned(),
                of: inner(n.stored()),
            },
            Shape::List(s) => ShapeNode::List { of: inner(s()) },
            Shape::Map(s) => ShapeNode::Map { of: inner(s()) },
            Shape::Optional(s) => ShapeNode::Optional { of: inner(s()) },
            Shape::Struct(s) => DefinitionDoc::define_struct(s, defs),
            Shape::Enum(e) => DefinitionDoc::define_enum(e, defs),
        }
    }
}
