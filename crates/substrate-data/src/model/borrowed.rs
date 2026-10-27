//! [`ValueRef`]: a value borrowed from where it was read — what schemas
//! read. Decoded from the binary form ([`crate::binary::decode_ref`]) no
//! string is copied: every one points into the bytes, and lists and maps
//! are slices in an arena ([`Bump`]). An owned [`Value`] lends itself the
//! same way ([`Value::borrow`]).

use bumpalo::Bump;

use crate::model::number::{int_to_f64, whole_to_i64};
use crate::model::value::{KindName, Map, Ref, RefKey, Value, ValueKind};

/// A borrowed [`Value`]: the same cases, strings `&'a str`, lists and maps slices.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ValueRef<'a> {
    /// Nothing.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// An integer.
    Int(i64),
    /// A number with a fraction or an exponent.
    Float(f64),
    /// A string, as spelled.
    Str(&'a str),
    /// A list, in the arena.
    List(&'a [ValueRef<'a>]),
    /// A map, in the arena.
    Map(MapRef<'a>),
    /// `@file:images/logo.png`: its kind checked, its id as written
    /// (checked where it was read).
    Ref {
        /// Its kind.
        kind: KindName,
        /// Its id, as written.
        id: &'a str,
    },
    /// Bytes as they are.
    Bytes(&'a [u8]),
}

/// A borrowed map: entries in file order.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MapRef<'a>(pub &'a [(&'a str, ValueRef<'a>)]);

impl<'a> MapRef<'a> {
    /// The value of `key`.
    #[must_use]
    pub fn get(self, key: &str) -> Option<&'a ValueRef<'a>> {
        self.0.iter().find(|(k, _)| *k == key).map(|(_, v)| v)
    }

    /// The entries, in order.
    #[must_use = "an iterator does nothing until it is consumed"]
    pub fn iter(self) -> impl ExactSizeIterator<Item = (&'a str, &'a ValueRef<'a>)> + 'a {
        self.0.iter().map(|(k, v)| (*k, v))
    }

    /// The keys, in order.
    #[must_use = "an iterator does nothing until it is consumed"]
    pub fn keys(self) -> impl ExactSizeIterator<Item = &'a str> + 'a {
        self.0.iter().map(|(k, _)| *k)
    }

    /// How many entries.
    #[must_use]
    pub fn len(self) -> usize {
        self.0.len()
    }

    /// Whether it has no entries.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.0.is_empty()
    }
}

impl<'a> ValueRef<'a> {
    /// Which case it is.
    #[must_use]
    pub fn kind(&self) -> ValueKind {
        match self {
            ValueRef::Null => ValueKind::Null,
            ValueRef::Bool(_) => ValueKind::Bool,
            ValueRef::Int(_) => ValueKind::Int,
            ValueRef::Float(_) => ValueKind::Float,
            ValueRef::Str(_) => ValueKind::Str,
            ValueRef::List(_) => ValueKind::List,
            ValueRef::Map(_) => ValueKind::Map,
            ValueRef::Ref { .. } => ValueKind::Ref,
            ValueRef::Bytes(_) => ValueKind::Bytes,
        }
    }

    /// Whether it's null.
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, ValueRef::Null)
    }

    /// The bool, if it's one.
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ValueRef::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// The integer, if it's one (a float with no fraction is).
    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            ValueRef::Int(n) => Some(*n),
            ValueRef::Float(f) => whole_to_i64(*f),
            _ => None,
        }
    }

    /// The number, if it's one.
    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            ValueRef::Int(n) => Some(int_to_f64(*n)),
            ValueRef::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// The string, if it's one.
    #[must_use]
    pub fn as_str(&self) -> Option<&'a str> {
        match self {
            ValueRef::Str(s) => Some(s),
            _ => None,
        }
    }

    /// The list, if it's one.
    #[must_use]
    pub fn as_list(&self) -> Option<&'a [ValueRef<'a>]> {
        match self {
            ValueRef::List(l) => Some(l),
            _ => None,
        }
    }

    /// The map, if it's one.
    #[must_use]
    pub fn as_map(&self) -> Option<MapRef<'a>> {
        match self {
            ValueRef::Map(m) => Some(*m),
            _ => None,
        }
    }

    /// `self[key]` of a map, `None` for anything else.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&'a ValueRef<'a>> {
        self.as_map()?.get(key)
    }

    /// An owned copy.
    #[must_use]
    pub fn to_value(self) -> Value {
        match self {
            ValueRef::Null => Value::Null,
            ValueRef::Bool(b) => Value::Bool(b),
            ValueRef::Int(n) => Value::Int(n),
            ValueRef::Float(f) => Value::Float(f),
            ValueRef::Str(s) => Value::Str(s.to_string()),
            ValueRef::List(l) => Value::List(l.iter().map(|v| v.to_value()).collect()),
            ValueRef::Map(m) => {
                let mut out = Map::with_capacity(m.len());
                for (k, v) in m.iter() {
                    out.insert(k, v.to_value());
                }
                Value::Map(out)
            }
            ValueRef::Ref { kind, id } => Value::Ref(Ref::new(kind, RefKey::spelled(id))),
            ValueRef::Bytes(b) => Value::Bytes(b.to_vec()),
        }
    }

    /// The reference of a `Ref`, owned.
    #[must_use]
    pub fn to_ref(self) -> Option<Ref> {
        match self {
            ValueRef::Ref { kind, id } => Some(Ref::new(kind, RefKey::spelled(id))),
            _ => None,
        }
    }
}

impl Value {
    /// This value, borrowed (lists and maps in `arena`) — what schemas read.
    pub fn borrow<'a>(&'a self, arena: &'a Bump) -> ValueRef<'a> {
        match self {
            Value::Null => ValueRef::Null,
            Value::Bool(b) => ValueRef::Bool(*b),
            Value::Int(n) => ValueRef::Int(*n),
            Value::Float(f) => ValueRef::Float(*f),
            Value::Str(s) => ValueRef::Str(s),
            Value::List(l) => {
                ValueRef::List(arena.alloc_slice_fill_iter(l.iter().map(|v| v.borrow(arena))))
            }
            Value::Map(m) => ValueRef::Map(MapRef(
                arena.alloc_slice_fill_iter(m.iter().map(|(k, v)| (k, v.borrow(arena)))),
            )),
            Value::Ref(r) => ValueRef::Ref {
                kind: r.kind,
                id: r.id.as_str(),
            },
            Value::Bytes(b) => ValueRef::Bytes(b),
        }
    }
}
