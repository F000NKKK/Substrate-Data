//! Shapes: what a format looks like.

use super::named_type::NamedShape;

// ── Shapes ──────────────────────────────────────────────────────────────────

/// A shape, built on demand — so shapes of recursive types are fine.
pub type ShapeFn = fn() -> Shape;

/// What a format looks like: drives an editor's property grids and
/// pickers, completion, hover docs and the docs' field tables.
#[derive(Debug, Clone)]
pub enum Shape {
    /// Any value.
    Any,
    Bool,
    Int {
        min: i64,
        max: i64,
    },
    /// A number with a fraction, `bits` wide (32 or 64): how a frame packs it.
    Float {
        bits: u32,
    },
    /// Plain text.
    Str,
    /// A name that repeats ([`crate::Atom`]): text everywhere, its number in a frame.
    Atom,
    /// Bytes as they are.
    Bytes,
    /// A reference of one of `kinds` (`&[]`: any kind), written
    /// `@kind:id`, its id of shape `id` (text, a number, a `uid`).
    Ref {
        kinds: &'static [&'static str],
        id: ShapeFn,
    },
    /// A type of the format's own that tools may know by name (`color`,
    /// `date`, `money`), stored as another shape: a tool that knows the
    /// name shows its own editor, one that doesn't edits what it's stored as.
    Named(NamedShape),
    List(ShapeFn),
    /// An object of any keys, every value this shape.
    Map(ShapeFn),
    /// A value or null.
    Optional(ShapeFn),
    Struct(StructShape),
    Enum(EnumShape),
}

#[derive(Debug, Clone)]
pub struct StructShape {
    pub name: &'static str,
    pub doc: &'static str,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: &'static str,
    pub doc: &'static str,
    pub shape: ShapeFn,
    /// Must be written (no default).
    pub required: bool,
    /// The numbers it holds must be in `min..=max` (`#[data(range = "1..=99")]`).
    pub range: Option<(f64, f64)>,
}

#[derive(Debug, Clone)]
pub struct EnumShape {
    pub name: &'static str,
    pub doc: &'static str,
    /// The key naming the variant (`"type"`), `None` for an enum of names.
    pub tag: Option<&'static str>,
    pub variants: Vec<Variant>,
}

#[derive(Debug, Clone)]
pub struct Variant {
    pub name: &'static str,
    pub doc: &'static str,
    /// The variant's fields (none for a name).
    pub fields: Vec<Field>,
}

impl Shape {
    /// The shape of `T`, a type of the format's own ([`crate::schema::NamedType`]).
    pub fn named<T: super::NamedType>() -> Shape {
        Shape::Named(NamedShape::of::<T>())
    }

    /// The shape as data, for tools in other languages (an IDE, a doc generator).
    /// Structs and enums are written once in `defs` and referred to by name.
    pub fn describe(&self) -> super::ShapeDoc {
        super::ShapeDoc::of(self)
    }
}
