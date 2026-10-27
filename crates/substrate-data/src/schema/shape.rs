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
    /// `true` or `false`.
    Bool,
    /// An integer in `min..=max`.
    Int {
        /// The smallest it takes.
        min: i64,
        /// The biggest it takes.
        max: i64,
    },
    /// A number with a fraction, `bits` wide (32 or 64): how a frame packs it.
    Float {
        /// Its width: 32 or 64.
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
        /// The kinds it takes (any kind if empty), the first the one written.
        kinds: &'static [&'static str],
        /// The shape of its id.
        id: ShapeFn,
    },
    /// A type of the format's own that tools may know by name (`color`,
    /// `date`, `money`), stored as another shape: a tool that knows the
    /// name shows its own editor, one that doesn't edits what it's stored as.
    Named(NamedShape),
    /// A list of the shape.
    List(ShapeFn),
    /// An object of any keys, every value this shape.
    Map(ShapeFn),
    /// A value or null.
    Optional(ShapeFn),
    /// A struct: an object of its fields.
    Struct(StructShape),
    /// An enum: a name, or a tagged object.
    Enum(EnumShape),
}

/// A struct's shape: its fields.
#[derive(Debug, Clone)]
pub struct StructShape {
    /// Its name in the docs.
    pub name: &'static str,
    /// Its docs.
    pub doc: &'static str,
    /// Its fields, in order.
    pub fields: Vec<Field>,
}

/// A field of a struct (or of an enum's variant).
#[derive(Debug, Clone)]
pub struct Field {
    /// Its key.
    pub name: &'static str,
    /// Its docs.
    pub doc: &'static str,
    /// Its shape.
    pub shape: ShapeFn,
    /// Must be written (no default).
    pub required: bool,
    /// The numbers it holds must be in `min..=max` (`#[data(range = "1..=99")]`).
    pub range: Option<(f64, f64)>,
}

/// An enum's shape: its variants, and the key naming one if it's tagged.
#[derive(Debug, Clone)]
pub struct EnumShape {
    /// Its name in the docs.
    pub name: &'static str,
    /// Its docs.
    pub doc: &'static str,
    /// The key naming the variant (`"type"`), `None` for an enum of names.
    pub tag: Option<&'static str>,
    /// Its variants, in order.
    pub variants: Vec<Variant>,
}

/// A variant of an enum.
#[derive(Debug, Clone)]
pub struct Variant {
    /// Its name as written.
    pub name: &'static str,
    /// Its docs.
    pub doc: &'static str,
    /// The variant's fields (none for a name).
    pub fields: Vec<Field>,
}

impl Shape {
    /// The shape of `T`, a type of the format's own ([`crate::schema::NamedType`]).
    #[must_use]
    pub fn named<T: super::NamedType>() -> Shape {
        Shape::Named(NamedShape::of::<T>())
    }

    /// The shape as data, for tools in other languages (an IDE, a doc generator).
    /// Structs and enums are written once in `defs` and referred to by name.
    #[must_use]
    pub fn describe(&self) -> super::ShapeDoc {
        super::ShapeDoc::of(self)
    }
}
