//! [`Writer`] writing a whole [`Value`]: the form a value takes is chosen
//! from the value alone, so every writer (this one, a port's) writes the
//! same bytes for it — `docs/format.md`, "Binary", says how.

use super::layout::narrows;
use super::{Writer, tag, varint};
use crate::model::value::{Map, Value};

/// How a list of values is written.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ListForm {
    /// One tag per item.
    Items,
    /// Every item an int: packed.
    Ints,
    /// Every item a float: packed.
    Floats,
    /// Every item a map with the same keys: the keys once.
    Records,
}

impl ListForm {
    fn of(items: &[Value]) -> ListForm {
        match items.first() {
            Some(Value::Int(_)) if items.iter().all(|v| matches!(v, Value::Int(_))) => {
                ListForm::Ints
            }
            Some(Value::Float(_)) if items.iter().all(|v| matches!(v, Value::Float(_))) => {
                ListForm::Floats
            }
            Some(Value::Map(first))
                if !first.is_empty()
                    && items
                        .iter()
                        .all(|v| matches!(v, Value::Map(m) if m.keys().eq(first.keys()))) =>
            {
                ListForm::Records
            }
            _ => ListForm::Items,
        }
    }
}

impl Writer {
    /// A whole value. A list of ints or of floats is packed, a list of
    /// maps with one set of keys writes the keys once, a map's keys are a
    /// shape, a small int takes a byte.
    pub fn value(&mut self, v: &Value) -> &mut Self {
        match v {
            Value::Null => self.null(),
            Value::Bool(b) => self.bool(*b),
            Value::Int(n) => self.int(*n),
            Value::Float(f) => self.float(*f),
            Value::Str(s) => self.str(s),
            Value::List(items) => self.list_value(items),
            Value::Map(map) => self.map_value(map),
            Value::Ref(r) => {
                self.out.push(tag::REF);
                self.string(r.kind.as_str());
                self.string(r.id.as_str());
                self
            }
            Value::Bytes(b) => self.bytes(b),
        }
    }

    fn list_value(&mut self, items: &[Value]) -> &mut Self {
        match ListForm::of(items) {
            ListForm::Ints => self.ints(items.iter().map(|v| match v {
                Value::Int(n) => *n,
                _ => 0,
            })),
            ListForm::Floats => self.float_items(items),
            ListForm::Records => self.records_value(items),
            ListForm::Items => {
                self.list(items.len());
                for item in items {
                    self.value(item);
                }
                self
            }
        }
    }

    /// A list of floats: four bytes each when every one is an `f32` exactly.
    fn float_items(&mut self, items: &[Value]) -> &mut Self {
        let float = |v: &Value| match v {
            Value::Float(f) => *f,
            _ => 0.0,
        };
        let narrow = items.iter().all(|v| narrows(float(v)));
        self.out
            .push(if narrow { tag::FLOATS32 } else { tag::FLOATS });
        varint(&mut self.out, items.len() as u64);
        for v in items {
            let f = float(v);
            if narrow {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "every item narrows exactly, checked above"
                )]
                self.out.extend_from_slice(&(f as f32).to_le_bytes());
            } else {
                self.out.extend_from_slice(&f.to_le_bytes());
            }
        }
        self
    }

    /// A list of maps that all have the keys of the first: the keys once.
    fn records_value(&mut self, items: &[Value]) -> &mut Self {
        let Some(Value::Map(first)) = items.first() else {
            return self.list(0);
        };
        self.out.push(tag::RECORDS);
        varint(&mut self.out, items.len() as u64);
        self.shape(first.keys());
        for item in items {
            if let Value::Map(map) = item {
                self.values_of(map);
            }
        }
        self
    }

    fn map_value(&mut self, map: &Map) -> &mut Self {
        if map.is_empty() {
            return self.map(0);
        }
        self.out.push(tag::MAP_SHAPE);
        self.shape(map.keys());
        self.values_of(map);
        self
    }

    /// The values of `map`, in order, with no keys.
    fn values_of(&mut self, map: &Map) {
        for v in map.values() {
            self.value(v);
        }
    }
}
