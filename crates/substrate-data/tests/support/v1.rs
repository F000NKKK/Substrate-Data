//! The version 1 writer of the binary form, kept for tests only: the
//! crate writes version 2 and reads both, so this is what makes version 1
//! documents to read (the legacy conformance vectors, the upgrade property
//! test). It is the writer the crate had, simplified: one buffer, a string
//! table by a map, no shapes, no inline ints, tags `0..=14` only.

use std::collections::HashMap;

use substrate_data::Value;

/// Version 1's tags.
mod tag {
    pub(super) const NULL: u8 = 0;
    pub(super) const FALSE: u8 = 1;
    pub(super) const TRUE: u8 = 2;
    pub(super) const INT: u8 = 3;
    pub(super) const FLOAT: u8 = 4;
    pub(super) const STR: u8 = 5;
    pub(super) const LIST: u8 = 6;
    pub(super) const MAP: u8 = 7;
    pub(super) const REF: u8 = 8;
    pub(super) const BYTES: u8 = 9;
    pub(super) const INTS: u8 = 10;
    pub(super) const STREAM: u8 = 11;
    pub(super) const FLOAT32: u8 = 12;
    pub(super) const FLOATS: u8 = 13;
    pub(super) const FLOATS32: u8 = 14;
}

/// Writes a version 1 document (or, from [`V1Writer::body`], a body with no header).
#[derive(Debug, Default)]
pub(crate) struct V1Writer {
    out: Vec<u8>,
    strings: HashMap<String, usize>,
}

fn varint(out: &mut Vec<u8>, mut n: u64) {
    while n > 0x7f {
        out.push(n.to_le_bytes()[0] | 0x80);
        n >>= 7;
    }
    out.push(n.to_le_bytes()[0]);
}

fn zigzag(n: i64) -> u64 {
    ((n << 1) ^ (n >> 63)).cast_unsigned()
}

/// Whether `f` survives `f64 -> f32 -> f64` bit for bit.
fn fits_f32(f: f64) -> bool {
    #[expect(clippy::cast_possible_truncation, reason = "narrowed to check")]
    let narrowed = f as f32;
    f64::from(narrowed).to_bits() == f.to_bits()
}

impl V1Writer {
    /// A document: `SDAT`, version 1.
    pub(crate) fn new() -> Self {
        let mut w = V1Writer::default();
        w.out.extend_from_slice(b"SDAT");
        w.out.push(1);
        w
    }

    /// A body with no header (a stream's, a frame's tagged value).
    pub(crate) fn body() -> Self {
        V1Writer::default()
    }

    pub(crate) fn finish(self) -> Vec<u8> {
        self.out
    }

    pub(crate) fn null(&mut self) -> &mut Self {
        self.out.push(tag::NULL);
        self
    }

    pub(crate) fn bool(&mut self, b: bool) -> &mut Self {
        self.out.push(if b { tag::TRUE } else { tag::FALSE });
        self
    }

    pub(crate) fn int(&mut self, n: i64) -> &mut Self {
        self.out.push(tag::INT);
        varint(&mut self.out, zigzag(n));
        self
    }

    pub(crate) fn float(&mut self, f: f64) -> &mut Self {
        self.out.push(tag::FLOAT);
        self.out.extend_from_slice(&f.to_le_bytes());
        self
    }

    pub(crate) fn float32(&mut self, f: f32) -> &mut Self {
        self.out.push(tag::FLOAT32);
        self.out.extend_from_slice(&f.to_le_bytes());
        self
    }

    pub(crate) fn floats(&mut self, items: &[f64]) -> &mut Self {
        let narrow = items.iter().all(|&f| fits_f32(f));
        self.out
            .push(if narrow { tag::FLOATS32 } else { tag::FLOATS });
        varint(&mut self.out, items.len() as u64);
        for f in items {
            if narrow {
                #[expect(clippy::cast_possible_truncation, reason = "every item fits")]
                self.out.extend_from_slice(&(*f as f32).to_le_bytes());
            } else {
                self.out.extend_from_slice(&f.to_le_bytes());
            }
        }
        self
    }

    pub(crate) fn floats32(&mut self, items: &[f32]) -> &mut Self {
        self.out.push(tag::FLOATS32);
        varint(&mut self.out, items.len() as u64);
        for f in items {
            self.out.extend_from_slice(&f.to_le_bytes());
        }
        self
    }

    pub(crate) fn str(&mut self, s: &str) -> &mut Self {
        self.out.push(tag::STR);
        self.string(s);
        self
    }

    pub(crate) fn bytes(&mut self, b: &[u8]) -> &mut Self {
        self.out.push(tag::BYTES);
        varint(&mut self.out, b.len() as u64);
        self.out.extend_from_slice(b);
        self
    }

    pub(crate) fn list(&mut self, len: usize) -> &mut Self {
        self.out.push(tag::LIST);
        varint(&mut self.out, len as u64);
        self
    }

    pub(crate) fn ints(&mut self, items: &[i64]) -> &mut Self {
        self.out.push(tag::INTS);
        varint(&mut self.out, items.len() as u64);
        for &n in items {
            varint(&mut self.out, zigzag(n));
        }
        self
    }

    /// A stream whose `len` items are already written, as a body, in `items`.
    pub(crate) fn stream(&mut self, len: usize, items: &[u8]) -> &mut Self {
        let mut count = Vec::new();
        varint(&mut count, len as u64);
        self.out.push(tag::STREAM);
        let body = u32::try_from(count.len() + items.len()).expect("a small stream");
        self.out.extend_from_slice(&body.to_le_bytes());
        self.out.extend_from_slice(&count);
        self.out.extend_from_slice(items);
        self
    }

    pub(crate) fn reference(&mut self, kind: &str, id: &str) -> &mut Self {
        self.out.push(tag::REF);
        self.string(kind);
        self.string(id);
        self
    }

    pub(crate) fn map(&mut self, len: usize) -> &mut Self {
        self.out.push(tag::MAP);
        varint(&mut self.out, len as u64);
        self
    }

    pub(crate) fn key(&mut self, k: &str) -> &mut Self {
        self.string(k);
        self
    }

    /// A whole value, every item with a tag of its own (version 1 had no
    /// packing for a model value).
    pub(crate) fn value(&mut self, v: &Value) -> &mut Self {
        match v {
            Value::Null => self.null(),
            Value::Bool(b) => self.bool(*b),
            Value::Int(n) => self.int(*n),
            Value::Float(f) => self.float(*f),
            Value::Str(s) => self.str(s),
            Value::List(items) => {
                self.list(items.len());
                for item in items {
                    self.value(item);
                }
                self
            }
            Value::Map(map) => {
                self.map(map.len());
                for (key, item) in map.iter() {
                    self.key(key);
                    self.value(item);
                }
                self
            }
            Value::Ref(r) => self.reference(r.kind.as_str(), r.id.as_str()),
            Value::Bytes(b) => self.bytes(b),
        }
    }

    fn string(&mut self, s: &str) {
        if let Some(&i) = self.strings.get(s) {
            varint(&mut self.out, (i as u64) << 1);
            return;
        }
        varint(&mut self.out, ((s.len() as u64) << 1) | 1);
        self.out.extend_from_slice(s.as_bytes());
        let next = self.strings.len();
        self.strings.insert(s.to_owned(), next);
    }
}

/// `value` as a version 1 document: what `binary::encode` wrote before version 2.
pub(crate) fn encode(value: &Value) -> Vec<u8> {
    let mut w = V1Writer::new();
    w.value(value);
    w.finish()
}
