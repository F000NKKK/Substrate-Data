//! The model: a tree of typed values.

use std::fmt;

/// A value.
use crate::model::number::{int_to_f64, whole_to_i64};
///
/// The text form has no syntax of its own for the typed values (`Ref`,
/// `Bytes`): they're written as strings (`"@file:images/logo.png"`), so
/// every file stays JSON, and the schema of a field says what its string
/// means. The binary form keeps the type.
///
/// A `Str` holds a string *as spelled*: one that starts with `@` is a
/// typed value's spelling, read by the field's schema
/// ([`crate::typed::classify`]), and plain text starting with `@` is kept
/// with it doubled (`"@@everyone"`). Text and binary forms carry it as is;
/// `Schema::write` of a `String` doubles the `@`, its read undoes it.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Value {
    /// Nothing.
    #[default]
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// An integer.
    Int(i64),
    /// A number with a fraction or an exponent.
    Float(f64),
    /// A string, as spelled (a plain one's leading `@` doubled).
    Str(String),
    /// A list.
    List(Vec<Value>),
    /// A map, in file order.
    Map(Map),
    /// `@file:images/logo.png` — something of some kind, by id.
    Ref(Ref),
    /// Bytes as they are — a picture, a sound — written `@bytes:<base64>` as text.
    Bytes(Vec<u8>),
}

/// A reference: its kind (`user`, `doc`, `file`, or any other the
/// format names) and id (`images/logo.png`, `team:alice`). The id is
/// opaque here; what it names and whether it exists is the format's
/// business (a [`crate::Resolve`] checks it), and a typed field reads it
/// as its kind's id type ([`crate::Id`]).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Ref {
    /// What it names: `user`, `doc`, `file`.
    pub kind: KindName,
    /// Which one, as written.
    pub id: RefKey,
}

impl Ref {
    /// `@kind:id`.
    #[must_use]
    pub fn new(kind: KindName, id: RefKey) -> Self {
        Self { kind, id }
    }

    /// `kind` and `id` checked: `Ref::parse("file", "images/logo.png")`.
    ///
    /// # Errors
    ///
    /// [`crate::typed::TypedError`] when `kind` or `id` isn't one.
    pub fn parse(kind: &str, id: &str) -> Result<Self, crate::typed::TypedError> {
        let kind = KindName::new(kind)?;
        let id = RefKey::new(id).map_err(|_| crate::typed::TypedError::BadId {
            kind: Some(kind),
            id: id.to_owned(),
        })?;
        Ok(Self { kind, id })
    }

    /// A typed id of kind `K` as a reference.
    pub fn of<K: crate::Kind>(id: &K::Id) -> Self {
        Self {
            kind: K::kind_name(),
            id: RefKey::spelled(&crate::schema::RefId::spell(id)),
        }
    }
}

impl fmt::Display for Ref {
    /// `@kind:id`, as files write it.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}:{}", crate::names::TYPED, self.kind, self.id)
    }
}

pub use super::map::Map;
pub use super::reference::{KindName, RefKey};

/// Which of a [`Value`]'s cases a value is — what messages name
/// (`"a string"`, `"a list"`) when they say what was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueKind {
    /// Null.
    Null,
    /// A bool.
    Bool,
    /// An integer.
    Int,
    /// A float.
    Float,
    /// A string.
    Str,
    /// A list.
    List,
    /// A map.
    Map,
    /// A reference.
    Ref,
    /// Bytes.
    Bytes,
}

impl fmt::Display for ValueKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ValueKind::Null => "null",
            ValueKind::Bool => "true/false",
            ValueKind::Int => "an integer",
            ValueKind::Float => "a number",
            ValueKind::Str => "a string",
            ValueKind::List => "a list",
            ValueKind::Map => "an object",
            ValueKind::Ref => "a reference",
            ValueKind::Bytes => "bytes",
        })
    }
}

impl Value {
    /// Which case it is.
    pub fn kind(&self) -> ValueKind {
        match self {
            Value::Null => ValueKind::Null,
            Value::Bool(_) => ValueKind::Bool,
            Value::Int(_) => ValueKind::Int,
            Value::Float(_) => ValueKind::Float,
            Value::Str(_) => ValueKind::Str,
            Value::List(_) => ValueKind::List,
            Value::Map(_) => ValueKind::Map,
            Value::Ref(_) => ValueKind::Ref,
            Value::Bytes(_) => ValueKind::Bytes,
        }
    }

    /// Whether it's null.
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// The bool, if it's one.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// The integer, if it's one (a float with no fraction is).
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int(n) => Some(*n),
            Value::Float(f) => whole_to_i64(*f),
            _ => None,
        }
    }

    /// The number, if it's one.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Int(n) => Some(int_to_f64(*n)),
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// The string, if it's one.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    /// The list, if it's one.
    pub fn as_list(&self) -> Option<&[Value]> {
        match self {
            Value::List(l) => Some(l),
            _ => None,
        }
    }

    /// The map, if it's one.
    pub fn as_map(&self) -> Option<&Map> {
        match self {
            Value::Map(m) => Some(m),
            _ => None,
        }
    }

    /// `self[key]` of a map, `None` for anything else.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.as_map()?.get(key)
    }

    /// The value at `path`.
    pub fn at(&self, path: &Path) -> Option<&Value> {
        let mut v = self;
        for seg in path.segments() {
            v = match (seg, v) {
                (Seg::Key(k), Value::Map(m)) => m.get(k)?,
                (Seg::Index(i), Value::List(l)) => l.get(*i)?,
                _ => return None,
            };
        }
        Some(v)
    }

    /// The value `segs` lead to, to change.
    pub fn at_mut(&mut self, segs: &[Seg]) -> Option<&mut Value> {
        let mut v = self;
        for seg in segs {
            v = match (seg, v) {
                (Seg::Key(k), Value::Map(m)) => m.get_mut(k)?,
                (Seg::Index(i), Value::List(l)) => l.get_mut(*i)?,
                _ => return None,
            };
        }
        Some(v)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}
impl From<i64> for Value {
    fn from(n: i64) -> Self {
        Value::Int(n)
    }
}
impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Float(f)
    }
}
impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::Str(s.to_string())
    }
}
impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::Str(s)
    }
}
impl From<Vec<Value>> for Value {
    fn from(l: Vec<Value>) -> Self {
        Value::List(l)
    }
}
impl From<Map> for Value {
    fn from(m: Map) -> Self {
        Value::Map(m)
    }
}

/// One step of a [`Path`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, crate::Schema)]
#[data(tag = "type")]
pub enum Seg {
    /// A map's key.
    Key(String),
    /// A list's index.
    Index(usize),
}

/// Where a value is in a document: `tasks[2].parts[0].owner`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Path(Vec<Seg>);

impl Path {
    /// The path of the document's root.
    #[must_use]
    pub fn root() -> Self {
        Self(Vec::new())
    }

    /// Its steps, from the root.
    #[must_use]
    pub fn segments(&self) -> &[Seg] {
        &self.0
    }

    /// Whether it's the root.
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    /// One step further.
    pub fn push(&mut self, seg: Seg) {
        self.0.push(seg);
    }

    /// One step back.
    pub fn pop(&mut self) {
        self.0.pop();
    }

    /// This path, then `key`.
    #[must_use]
    pub fn key(&self, key: &str) -> Path {
        let mut p = self.clone();
        p.0.push(Seg::Key(key.to_string()));
        p
    }

    /// `rest` under this path.
    #[must_use]
    pub fn then(&self, rest: &Path) -> Path {
        let mut p = self.clone();
        p.0.extend(rest.0.iter().cloned());
        p
    }

    /// This path, then index `i`.
    #[must_use]
    pub fn index(&self, i: usize) -> Path {
        let mut p = self.clone();
        p.0.push(Seg::Index(i));
        p
    }
}

/// A path as its steps.
impl crate::Schema for Path {
    fn shape() -> crate::Shape {
        Vec::<Seg>::shape()
    }
    fn read(v: &crate::ValueRef<'_>, cx: &mut crate::Reader<'_>) -> Option<Self> {
        Vec::<Seg>::read(v, cx).map(Path)
    }
    fn write(&self) -> Value {
        self.0.write()
    }
    fn pack(&self, p: &mut crate::frame::Packer) {
        self.0.pack(p);
    }
    fn unpack(u: &mut crate::frame::Unpacker<'_>) -> Option<Self> {
        Vec::<Seg>::unpack(u).map(Path)
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return f.write_str("(root)");
        }
        for (i, seg) in self.0.iter().enumerate() {
            match seg {
                Seg::Key(k) if is_plain_key(k) => {
                    if i > 0 {
                        f.write_str(".")?;
                    }
                    f.write_str(k)?;
                }
                Seg::Key(k) => write!(f, "[{k:?}]")?,
                Seg::Index(n) => write!(f, "[{n}]")?,
            }
        }
        Ok(())
    }
}

fn is_plain_key(k: &str) -> bool {
    let mut chars = k.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}
