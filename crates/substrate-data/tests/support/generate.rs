//! Random documents of version 1 and the value each one holds: every tag
//! version 1 has (the packed lists, the single `f32`, streams with a table
//! of their own), every value it can hold (an int at the edges, a float of
//! any bits, strings that look like references, keys in every order). The
//! same documents on every run.

use std::collections::BTreeMap;

use substrate_data::{Map, Ref, Value};

use crate::v1::V1Writer;

/// A linear congruential generator: the same documents on every run. It
/// counts what it has generated, so a test can say it covered everything.
pub(crate) struct Rng(u64, pub(crate) BTreeMap<&'static str, usize>);

impl Rng {
    pub(crate) fn new(seed: u64) -> Rng {
        Rng(seed, BTreeMap::new())
    }

    pub(crate) fn note(&mut self, what: &'static str) {
        *self.1.entry(what).or_default() += 1;
    }

    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 24
    }

    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % n as u64).expect("a small number")
    }
}

const WORDS: [&str; 12] = [
    "name",
    "id",
    "kind",
    "x",
    "y",
    "tags",
    "owner",
    "@@at",
    "@file:x",
    "é",
    "",
    "a longer word",
];
const KINDS: [&str; 4] = ["file", "user", "doc", "project_2"];

fn int(rng: &mut Rng) -> i64 {
    match rng.below(6) {
        0 => i64::try_from(rng.below(300)).expect("small") - 20,
        1 => [-17, -16, -15, 110, 111, 112, 127, 128, 0, 1][rng.below(10)],
        2 => i64::MIN + i64::try_from(rng.below(3)).expect("small"),
        3 => i64::MAX - i64::try_from(rng.below(3)).expect("small"),
        _ => rng.next().cast_signed(),
    }
}

fn float(rng: &mut Rng) -> f64 {
    match rng.below(6) {
        0 => f64::from_bits(rng.next() << 20 | rng.next()),
        1 => [
            0.0,
            -0.0,
            0.5,
            1.0,
            -1.5,
            f64::INFINITY,
            f64::NEG_INFINITY,
            1e16,
        ][rng.below(8)],
        2 => f64::from(f32::from_bits(
            u32::try_from(rng.next() & 0xffff_ffff).expect("32 bits"),
        )),
        3 => f64::NAN,
        4 => f64::from_bits(0x7ff0_0000_0000_0001 + rng.next() % 1000),
        _ => f64::from(u32::try_from(rng.next() % 1000).expect("small")) / 8.0,
    }
}

fn word(rng: &mut Rng) -> String {
    if rng.below(8) == 0 {
        "w".repeat(rng.below(300))
    } else {
        WORDS[rng.below(WORDS.len())].to_owned()
    }
}

/// Writes a random value into `w` the way version 1 writes it, and says what it is.
fn generate(rng: &mut Rng, w: &mut V1Writer, depth: usize) -> Value {
    let top = if depth == 0 { 8 } else { 15 };
    match rng.below(top) {
        0 => {
            rng.note("null");
            w.null();
            Value::Null
        }
        1 => {
            let b = rng.below(2) == 0;
            w.bool(b);
            Value::Bool(b)
        }
        2 => {
            let n = int(rng);
            w.int(n);
            Value::Int(n)
        }
        3 => {
            let f = float(rng);
            w.float(f);
            Value::Float(f)
        }
        4 => {
            rng.note("float32");
            let f = f32::from_bits(u32::try_from(rng.next() & 0xffff_ffff).expect("32 bits"));
            w.float32(f);
            Value::Float(f64::from(f))
        }
        5 => {
            let s = word(rng);
            w.str(&s);
            Value::Str(s)
        }
        6 | 7 => bytes_or_ref(rng, w),
        _ => composite(rng, w, depth),
    }
}

fn bytes_or_ref(rng: &mut Rng, w: &mut V1Writer) -> Value {
    if rng.below(2) == 0 {
        rng.note("bytes");
        let bytes: Vec<u8> = (0..rng.below(20))
            .map(|_| rng.next().to_le_bytes()[0])
            .collect();
        w.bytes(&bytes);
        return Value::Bytes(bytes);
    }
    rng.note("ref");
    let kind = KINDS[rng.below(KINDS.len())];
    let id = format!("{}:{}", word(rng).replace(' ', "_"), rng.below(5));
    w.reference(kind, &id);
    Value::Ref(Ref::parse(kind, &id).expect("a reference"))
}

fn composite(rng: &mut Rng, w: &mut V1Writer, depth: usize) -> Value {
    match rng.below(8) {
        7 => records(rng, w, depth),
        0 => {
            let n = rng.below(5);
            w.list(n);
            Value::List((0..n).map(|_| generate(rng, w, depth - 1)).collect())
        }
        1 => {
            rng.note("ints");
            let items: Vec<i64> = (0..rng.below(8)).map(|_| int(rng)).collect();
            w.ints(&items);
            Value::List(items.into_iter().map(Value::Int).collect())
        }
        2 => {
            rng.note("floats");
            let items: Vec<f64> = (0..rng.below(8)).map(|_| float(rng)).collect();
            w.floats(&items);
            Value::List(items.into_iter().map(Value::Float).collect())
        }
        3 => {
            rng.note("floats32");
            let items: Vec<f32> = (0..rng.below(8))
                .map(|_| f32::from_bits(u32::try_from(rng.next() & 0xffff_ffff).expect("32 bits")))
                .collect();
            w.floats32(&items);
            Value::List(
                items
                    .into_iter()
                    .map(|f| Value::Float(f64::from(f)))
                    .collect(),
            )
        }
        4 => stream(rng, w, depth),
        _ => map(rng, w, depth),
    }
}

/// A list of maps with the same keys (in one order, now and then another).
fn records(rng: &mut Rng, w: &mut V1Writer, depth: usize) -> Value {
    rng.note("records");
    let mut keys: Vec<String> = Vec::new();
    let width = 1 + rng.below(4);
    while keys.len() < width {
        let key = word(rng);
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    let count = 1 + rng.below(5);
    w.list(count);
    let mut items = Vec::new();
    for _ in 0..count {
        let mut order = keys.clone();
        if rng.below(6) == 0 {
            order.reverse();
        }
        w.map(order.len());
        let mut m = Map::new();
        for key in order {
            w.key(&key);
            m.insert(key, generate(rng, w, depth - 1));
        }
        items.push(Value::Map(m));
    }
    Value::List(items)
}

/// A stream: its items written apart, with a string table of their own.
fn stream(rng: &mut Rng, w: &mut V1Writer, depth: usize) -> Value {
    rng.note("stream");
    let mut body = V1Writer::body();
    let items: Vec<Value> = (0..rng.below(4))
        .map(|_| generate(rng, &mut body, depth - 1))
        .collect();
    w.stream(items.len(), &body.finish());
    Value::List(items)
}

fn map(rng: &mut Rng, w: &mut V1Writer, depth: usize) -> Value {
    rng.note("map");
    let mut keys: Vec<String> = Vec::new();
    let n = if rng.below(10) == 0 {
        18 + rng.below(8)
    } else {
        rng.below(5)
    };
    while keys.len() < n {
        let key = if n > 16 {
            format!("k{}", keys.len())
        } else {
            word(rng)
        };
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    w.map(keys.len());
    let mut m = Map::new();
    for key in keys {
        w.key(&key);
        m.insert(key, generate(rng, w, depth - 1));
    }
    Value::Map(m)
}

/// A random document in version 1, and the value it holds.
pub(crate) fn document(rng: &mut Rng) -> (Vec<u8>, Value) {
    let mut w = V1Writer::new();
    let value = generate(rng, &mut w, 4);
    (w.finish(), value)
}
