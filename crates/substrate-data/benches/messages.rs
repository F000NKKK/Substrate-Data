//! The path for typed messages: a struct written to the binary
//! form (`write` + `binary::encode`) and read back (`read_bytes`), for a
//! small, a medium and a large message — against a hand-written byte
//! layout of the same data, the ceiling for any encoding. And sending one
//! as a frame: `write_frame` against the reusable `FrameWriter`.

#![expect(
    clippy::unwrap_used,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "a benchmark: fixed inputs that always decode, generated numbers in range"
)]

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use substrate_data::Schema;

#[derive(Schema, Clone, PartialEq, Debug)]
struct Point {
    x: f64,
    y: f64,
    z: f64,
}

/// Small: a user's profile.
#[derive(Schema, Clone, PartialEq, Debug)]
struct UserCard {
    role_id: String,
    region: String,
    home: Point,
    score: f64,
    rating: f64,
    weight: f64,
    flags: u32,
    name: String,
}

#[derive(Schema, Clone, PartialEq, Debug)]
struct Line {
    sku: String,
    count: u32,
    extra: String,
}

/// Medium: a shopping cart.
#[derive(Schema, Clone, PartialEq, Debug)]
struct CartLine {
    position: u32,
    line: Line,
}

#[derive(Schema, Clone, PartialEq, Debug)]
struct Cart {
    lines: Vec<CartLine>,
}

/// Large: a 32×32×32 table of codes.
#[derive(Schema, Clone, PartialEq, Debug)]
struct Grid {
    legend: Vec<String>,
    cells: Vec<u16>,
}

/// Strings: each different (the writer's hash index), or a few over and over.
#[derive(Schema, Clone, PartialEq, Debug)]
struct Strings {
    items: Vec<String>,
}

/// Nested: a chain of optional boxes.
#[derive(Schema, Clone, PartialEq, Debug)]
struct Node {
    value: i32,
    next: Option<Box<Node>>,
}

/// Nothing at all.
#[derive(Schema, Clone, PartialEq, Debug)]
struct Empty {}

/// Floats: not packed, eight bytes each.
#[derive(Schema, Clone, PartialEq, Debug)]
struct Floats {
    items: Vec<f64>,
}

/// Bytes as they are: a picture, a sound.
#[derive(Schema, Clone, PartialEq, Debug)]
struct Blob {
    #[data(with = "substrate_data::with::Raw")]
    bytes: Vec<u8>,
}

/// A big list of structs: whole (`Vec`), or kept written (`Stream`, read when asked).
#[derive(Schema, Clone, PartialEq, Debug)]
struct Entry {
    id: String,
    count: u32,
}

#[derive(Schema, Clone, PartialEq, Debug)]
struct Entries {
    items: Vec<Entry>,
}

#[derive(Schema, Clone, PartialEq, Debug)]
struct EntryStream {
    items: substrate_data::Stream<Entry>,
}

const ENTRIES: usize = 10_000;

fn entries() -> Vec<Entry> {
    (0..ENTRIES)
        .map(|i| Entry {
            id: format!("docs/page_{}", i % 16),
            count: i as u32,
        })
        .collect()
}

const MANY: usize = 2000;
const FEW_KINDS: usize = 8;
const DEPTH: i32 = 50;
const BLOB_BYTES: usize = 1 << 16;
const FLOATS: usize = 10_000;

fn strings(distinct: usize) -> Strings {
    Strings {
        items: (0..MANY)
            .map(|i| format!("core:thing_{}", i % distinct))
            .collect(),
    }
}

fn chain() -> Node {
    (0..DEPTH).fold(
        Node {
            value: DEPTH,
            next: None,
        },
        |next, value| Node {
            value,
            next: Some(Box::new(next)),
        },
    )
}

const GRID_SIDE: usize = 32;
const CART_LINES: u32 = 41;

fn user() -> UserCard {
    UserCard {
        role_id: "core:editor".into(),
        region: "core:eu_west".into(),
        home: Point {
            x: 12.5,
            y: 64.0,
            z: -3.25,
        },
        score: 1.74,
        rating: 0.6,
        weight: 1.95,
        flags: 169,
        name: "Alice".into(),
    }
}

fn cart() -> Cart {
    Cart {
        lines: (0..CART_LINES)
            .map(|position| CartLine {
                position,
                line: Line {
                    sku: format!("core:sku_{}", position % 7),
                    count: position % 64 + 1,
                    extra: "{}".into(),
                },
            })
            .collect(),
    }
}

fn grid() -> Grid {
    let n = GRID_SIDE * GRID_SIDE * GRID_SIDE;
    Grid {
        legend: vec!["core:low".into(), "core:none".into(), "core:high".into()],
        cells: (0..n).map(|i| (i % 3) as u16).collect(),
    }
}

fn encode<T: Schema>(v: &T) -> Vec<u8> {
    substrate_data::to_bytes(v)
}

/// The way through a value tree: what `encode` was before it went straight.
fn encode_via_value<T: Schema>(v: &T) -> Vec<u8> {
    substrate_data::binary::encode(&v.write())
}

/// The hand-written layout: little-endian numbers, strings a u16 length.
mod raw {
    pub(crate) fn str(o: &mut Vec<u8>, s: &str) {
        o.extend_from_slice(&(s.len() as u16).to_le_bytes());
        o.extend_from_slice(s.as_bytes());
    }

    pub(crate) fn user(e: &super::UserCard) -> Vec<u8> {
        let mut o = Vec::with_capacity(96);
        str(&mut o, &e.role_id);
        str(&mut o, &e.region);
        for f in [e.home.x, e.home.y, e.home.z, e.score, e.rating, e.weight] {
            o.extend_from_slice(&f.to_le_bytes());
        }
        o.extend_from_slice(&e.flags.to_le_bytes());
        str(&mut o, &e.name);
        o
    }

    pub(crate) fn grid(a: &super::Grid) -> Vec<u8> {
        let mut o = Vec::with_capacity(a.cells.len() * 2 + 64);
        for p in &a.legend {
            str(&mut o, p);
        }
        for b in &a.cells {
            o.extend_from_slice(&b.to_le_bytes());
        }
        o
    }
}

fn case<T: Schema + PartialEq + std::fmt::Debug>(
    c: &mut Criterion,
    name: &str,
    v: &T,
    raw: Option<fn(&T) -> Vec<u8>>,
) {
    let bytes = encode(v);
    assert_eq!(&substrate_data::read_bytes::<T>(&bytes).unwrap(), v);
    let mut g = c.benchmark_group(name);
    g.throughput(Throughput::Bytes(bytes.len() as u64));
    g.bench_function(BenchmarkId::new("encode", bytes.len()), |b| {
        b.iter(|| encode(black_box(v)));
    });
    g.bench_function(BenchmarkId::new("encode via value", bytes.len()), |b| {
        b.iter(|| encode_via_value(black_box(v)));
    });
    g.bench_function(BenchmarkId::new("decode", bytes.len()), |b| {
        b.iter(|| substrate_data::read_bytes::<T>(black_box(&bytes)).unwrap());
    });
    g.bench_function(BenchmarkId::new("decode via value", bytes.len()), |b| {
        b.iter(|| {
            let arena = substrate_data::Bump::new();
            substrate_data::read_ref_value::<T>(
                &substrate_data::binary::decode_ref(black_box(&bytes), &arena).unwrap(),
            )
            .unwrap()
        });
    });
    if let Some(raw) = raw {
        g.bench_function(BenchmarkId::new("raw encode", raw(v).len()), |b| {
            b.iter(|| raw(black_box(v)));
        });
    }
    g.finish();
}

/// A writer that keeps nothing: what a send costs, not what a socket does.
struct Discard;

impl std::io::Write for Discard {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(black_box(buf).len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// One message sent as a frame three ways.
fn send_case<T: Schema>(c: &mut Criterion, name: &str, v: &T) {
    use substrate_data::io::{FrameWriter, write_frame};
    let mut g = c.benchmark_group(format!("send/{name}"));
    g.throughput(Throughput::Bytes(encode(v).len() as u64));
    g.bench_function("write_frame", |b| {
        b.iter(|| write_frame(&mut Discard, black_box(v)).unwrap());
    });
    let mut sender = FrameWriter::new();
    g.bench_function("FrameWriter::send", |b| {
        b.iter(|| sender.send(&mut Discard, black_box(v)).unwrap());
    });
    g.bench_function("FrameWriter::send_packed", |b| {
        b.iter(|| sender.send_packed(&mut Discard, black_box(v)).unwrap());
    });
    g.finish();
}

fn bench_send(c: &mut Criterion) {
    send_case(c, "small_user_info", &user());
    send_case(c, "medium_cart", &cart());
    send_case(c, "large_grid_32", &grid());
}

fn bench(c: &mut Criterion) {
    case(c, "user_info", &user(), Some(raw::user));
    case(c, "cart", &cart(), None);
    case(c, "grid_32", &grid(), Some(raw::grid));
    case(c, "entries_vec_10k", &Entries { items: entries() }, None);
    let stream = EntryStream {
        items: entries().into_iter().collect(),
    };
    case(c, "entries_stream_10k", &stream, None);
    c.bench_function("entries_stream_10k/build from items", |b| {
        b.iter(|| {
            entries()
                .into_iter()
                .collect::<substrate_data::Stream<Entry>>()
        });
    });
    c.bench_function("entries_vec_10k/build from items", |b| b.iter(entries));
    c.bench_function("entries_stream_10k/read all items", |b| {
        b.iter(|| black_box(&stream).items.iter().flatten().count());
    });
    case(c, "strings_unique", &strings(MANY), None);
    case(c, "strings_repeated", &strings(FEW_KINDS), None);
    case(c, "nested_50", &chain(), None);
    case(c, "empty", &Empty {}, None);
    case(
        c,
        "floats_10k",
        &Floats {
            items: (0..FLOATS).map(|i| i as f64 * 0.5).collect(),
        },
        None,
    );
    case(
        c,
        "bytes_64k",
        &Blob {
            bytes: (0..BLOB_BYTES).map(|i| i as u8).collect(),
        },
        None,
    );
    // A message with a field the type doesn't know: read the slow way, to say so.
    let mut unknown = user().write();
    if let substrate_data::Value::Map(m) = &mut unknown {
        m.insert("unknown", substrate_data::Value::Int(1));
    }
    let bytes = substrate_data::binary::encode(&unknown);
    c.bench_function("unknown_field/decode (fails, says why)", |b| {
        b.iter(|| substrate_data::read_bytes::<UserCard>(black_box(&bytes)).unwrap_err());
    });
}

criterion_group!(benches, bench, bench_send);
criterion_main!(benches);
