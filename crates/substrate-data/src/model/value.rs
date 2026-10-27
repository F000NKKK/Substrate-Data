//! The model: a tree of typed values.

use std::fmt;

/// A value.
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
    #[default]
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<Value>),
    Map(Map),
    /// `@file:images/logo.png` — something of some kind, by id.
    Ref(Ref),
    /// Bytes as they are — a picture, a sound — written `@bytes:<base64>` as text.
    Bytes(Vec<u8>),
}

/// A reference: its kind (`user`, `doc`, `file`, or any other the
/// format names) and id (`images/logo.png`, `team:alice`). The id is
/// opaque here; what it names and whether it exists is the format's
/// business (a [`crate::Resolve`] checks it).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Ref {
    pub kind: String,
    pub id: String,
}

impl Ref {
    pub fn new(kind: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            id: id.into(),
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

impl Value {
    /// The type's name, for messages: `"a string"`, `"a list"`, ...
    pub fn kind_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "true/false",
            Value::Int(_) => "an integer",
            Value::Float(_) => "a number",
            Value::Str(_) => "a string",
            Value::List(_) => "a list",
            Value::Map(_) => "an object",
            Value::Ref(_) => "a reference",
            Value::Bytes(_) => "bytes",
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int(n) => Some(*n),
            Value::Float(f) if f.fract() == 0.0 && f.abs() < 9.0e15 => Some(*f as i64),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Int(n) => Some(*n as f64),
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[Value]> {
        match self {
            Value::List(l) => Some(l),
            _ => None,
        }
    }

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
    Key(String),
    Index(usize),
}

/// Where a value is in a document: `tasks[2].parts[0].owner`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Path(Vec<Seg>);

impl Path {
    pub fn root() -> Self {
        Self(Vec::new())
    }

    pub fn segments(&self) -> &[Seg] {
        &self.0
    }

    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    pub fn push(&mut self, seg: Seg) {
        self.0.push(seg);
    }

    pub fn pop(&mut self) {
        self.0.pop();
    }

    pub fn key(&self, key: &str) -> Path {
        let mut p = self.clone();
        p.0.push(Seg::Key(key.to_string()));
        p
    }

    /// `rest` under this path.
    pub fn then(&self, rest: &Path) -> Path {
        let mut p = self.clone();
        p.0.extend(rest.0.iter().cloned());
        p
    }

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
    fn read(v: &crate::ValueRef<'_>, cx: &mut crate::Reader) -> Option<Self> {
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
