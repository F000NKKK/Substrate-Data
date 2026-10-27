//! Frames, the packed form for wire messages: a typed document packed,
//! unpacked, read as a view or by its shape alone; the `Packer` and
//! `Unpacker` a schema drives; and the float kernels. One group per path;
//! throughput is the bytes of the frame. The `io` streams are in `io.rs`.

#![expect(clippy::unwrap_used, reason = "a benchmark: the corpus always unpacks")]

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use substrate_data::Schema;
use substrate_data::frame::{self, Packer, Unpacker, floats};

#[expect(dead_code, reason = "the corpus is shared; this bench uses part of it")]
mod corpus;

use corpus::each::{group, quick};
use corpus::sample::Size;
use corpus::typed::{DocRecord, Docs, Numbers, Typed, Users, Visit, docs, each, numbers, users};

/// Every typed document through the frame paths that take any `Schema`.
struct Whole<'a>(&'a mut Criterion);

impl Visit for Whole<'_> {
    fn visit<T: Typed>(&mut self, name: &str, value: &T) {
        let bytes = frame::to_frame(value);
        assert_eq!(&frame::read_frame::<T>(&bytes).unwrap(), value);
        let shape = T::shape();
        let mut packer = Packer::new();
        let mut g = group(self.0, "frame/whole");
        g.throughput(Throughput::Bytes(bytes.len() as u64));
        g.bench_function(format!("to_frame/{name}"), |b| {
            b.iter(|| frame::to_frame(black_box(value)));
        });
        g.bench_function(format!("with_frame_reused/{name}"), |b| {
            b.iter(|| packer.with_frame(black_box(value), <[u8]>::len));
        });
        g.bench_function(format!("read_frame/{name}"), |b| {
            b.iter(|| frame::read_frame::<T>(black_box(&bytes)).unwrap());
        });
        g.bench_function(format!("read_shaped/{name}"), |b| {
            b.iter(|| frame::read_shaped(black_box(&bytes), &shape).unwrap());
        });
        g.finish();
    }
}

/// Views of documents: the frame read in place, nothing copied.
fn views(c: &mut Criterion) {
    let mut g = group(c, "frame/view");
    for (label, n) in [("small", 1), ("medium", 30), ("large", 2900)] {
        let bytes = frame::to_frame(&docs(n));
        g.throughput(Throughput::Bytes(bytes.len() as u64));
        g.bench_function(format!("docs_read_view/{label}"), |b| {
            b.iter(|| {
                frame::read_view::<Docs>(black_box(&bytes))
                    .unwrap()
                    .docs
                    .len()
            });
        });
        g.bench_function(format!("docs_walk_view/{label}"), |b| {
            b.iter(|| {
                let view = frame::read_view::<Docs>(black_box(&bytes)).unwrap();
                view.docs
                    .iter()
                    .map(|d| d.title.len() + d.tags.len())
                    .sum::<usize>()
            });
        });
    }
    let bytes = frame::to_frame(&users(40));
    g.throughput(Throughput::Bytes(bytes.len() as u64));
    g.bench_function("users_read_view/medium", |b| {
        b.iter(|| {
            frame::read_view::<Users>(black_box(&bytes))
                .unwrap()
                .users
                .len()
        });
    });
    g.finish();
    number_views(c);
}

/// Views of lists of numbers: read in place, no alignment asked.
fn number_views(c: &mut Criterion) {
    let mut g = group(c, "frame/view");
    for (label, n) in [("small", 1), ("medium", 100), ("large", 10_000)] {
        let bytes = frame::to_frame(&numbers(n));
        g.throughput(Throughput::Bytes(bytes.len() as u64));
        g.bench_function(format!("numbers_sum_ids/{label}"), |b| {
            b.iter(|| {
                let v = frame::read_view::<Numbers>(black_box(&bytes)).unwrap();
                v.ids.iter().map(u64::from).sum::<u64>()
            });
        });
        g.bench_function(format!("numbers_get_one/{label}"), |b| {
            b.iter(|| {
                frame::read_view::<Numbers>(black_box(&bytes))
                    .unwrap()
                    .ids
                    .get(0)
            });
        });
    }
    g.finish();
}

/// What a schema's `pack` does: records field by field.
fn pack_records(items: &[DocRecord]) -> Vec<u8> {
    let mut p = Packer::new();
    let list = p.begin_list(items.len());
    for d in items {
        let at = p.begin();
        p.put_u(u64::from(d.id));
        p.put_str(&d.title);
        p.put_str(&d.author);
        p.put_f64(d.rating);
        p.put_bool(d.public);
        p.put_uints(d.tags.len(), d.tags.iter().map(|t| t.len() as u64));
        p.end(at);
    }
    p.end(list);
    p.finish()
}

fn unpack_records(bytes: &[u8]) -> usize {
    let mut u = Unpacker::open(bytes).unwrap();
    let (n, list_end) = u.list().unwrap();
    let mut total = 0;
    for _ in 0..n {
        let end = u.begin().unwrap();
        total += usize::try_from(u.uint().unwrap()).unwrap();
        total += u.str().unwrap().len() + u.str().unwrap().len();
        total += usize::from(black_box(u.f64().unwrap()) > 1.0) + usize::from(u.bool().unwrap());
        total += u.uints(Some).unwrap().len();
        u.leave(end).unwrap();
    }
    u.skip_to(list_end).unwrap();
    total + usize::from(u.at_end())
}

fn steps(c: &mut Criterion) {
    let mut g = group(c, "frame/steps");
    for (label, n) in [("small", 1), ("medium", 30), ("large", 2900)] {
        let items = docs(n).docs;
        let bytes = pack_records(&items);
        g.throughput(Throughput::Bytes(bytes.len() as u64));
        g.bench_function(format!("packer_records/{label}"), |b| {
            b.iter(|| pack_records(black_box(&items)));
        });
        g.bench_function(format!("unpacker_records/{label}"), |b| {
            b.iter(|| unpack_records(black_box(&bytes)));
        });
    }
    g.finish();
}

/// The lists the packed-list benches use: ten thousand of each.
struct Lists {
    ints: Vec<u64>,
    wide: Vec<f64>,
    narrow: Vec<f64>,
    singles: Vec<f32>,
}

impl Lists {
    const LEN: u32 = 10_000;

    fn new() -> Self {
        let n = Self::LEN;
        Self {
            ints: (0..u64::from(n)).map(|i| i * 7 % 5000).collect(),
            wide: (0..n).map(|i| f64::from(i) * 0.1 + 1e-9).collect(),
            narrow: (0..n).map(|i| f64::from(i) * 0.5).collect(),
            singles: (0..n)
                .map(|i| f32::from(u16::try_from(i % 60_000).unwrap()) * 0.25)
                .collect(),
        }
    }
}

fn packed<R>(fill: impl FnOnce(&mut Packer) -> R) -> Vec<u8> {
    let mut p = Packer::new();
    fill(&mut p);
    p.finish()
}

/// Lists of numbers and floats packed and read back.
fn numbers_and_floats(c: &mut Criterion) {
    let l = Lists::new();
    let mut g = group(c, "frame/lists");
    g.throughput(Throughput::Elements(u64::from(Lists::LEN)));
    g.bench_function("put_uints", |b| {
        b.iter(|| packed(|p| p.put_uints(l.ints.len(), l.ints.iter().copied())));
    });
    for (name, items) in [("put_f64s_wide", &l.wide), ("put_f64s_narrow", &l.narrow)] {
        g.bench_function(name, |b| {
            b.iter(|| packed(|p| p.put_f64s(black_box(items))));
        });
    }
    g.bench_function("put_f32s", |b| {
        b.iter(|| packed(|p| p.put_f32s(black_box(&l.singles))));
    });
    let ints = packed(|p| p.put_uints(l.ints.len(), l.ints.iter().copied()));
    g.bench_function("uints", |b| {
        b.iter(|| {
            Unpacker::open(black_box(&ints))
                .unwrap()
                .uints(Some)
                .unwrap()
        });
    });
    g.bench_function("packed_ints", |b| {
        b.iter(|| {
            Unpacker::open(black_box(&ints))
                .unwrap()
                .packed_ints()
                .unwrap()
                .1
        });
    });
    for (name, items) in [("f64s_wide", &l.wide), ("f64s_narrow", &l.narrow)] {
        let bytes = packed(|p| p.put_f64s(items));
        g.bench_function(format!("read_{name}"), |b| {
            b.iter(|| Unpacker::open(black_box(&bytes)).unwrap().f64s().unwrap());
        });
        g.bench_function(format!("packed_floats_{name}"), |b| {
            b.iter(|| {
                Unpacker::open(black_box(&bytes))
                    .unwrap()
                    .packed_floats(true)
                    .is_some()
            });
        });
    }
    g.finish();
    float_kernels(c, &l.wide, &l.narrow, &l.singles);
}

fn float_kernels(c: &mut Criterion, wide: &[f64], narrow: &[f64], singles: &[f32]) {
    let mut g = group(c, "frame/floats");
    g.throughput(Throughput::Elements(wide.len() as u64));
    g.bench_function("fits_f32_yes", |b| {
        b.iter(|| floats::fits_f32(black_box(narrow)));
    });
    g.bench_function("fits_f32_no", |b| {
        b.iter(|| floats::fits_f32(black_box(wide)));
    });
    let mut out = Vec::new();
    g.bench_function("put_narrow", |b| {
        b.iter(|| {
            out.clear();
            floats::put_narrow(&mut out, black_box(narrow));
        });
    });
    g.bench_function("put_wide", |b| {
        b.iter(|| {
            out.clear();
            floats::put_wide(&mut out, black_box(wide));
        });
    });
    g.bench_function("put_f32s", |b| {
        b.iter(|| {
            out.clear();
            floats::put_f32s(&mut out, black_box(singles));
        });
    });
    let mut raw = Vec::new();
    floats::put_narrow(&mut raw, narrow);
    g.bench_function("widen", |b| b.iter(|| floats::widen(black_box(&raw))));
    g.finish();
}

/// A thousand records of the scalars: signed, `f32`, bytes, a tagged value, a shared string.
fn pack_scalars(n: u32) -> Vec<u8> {
    let mut p = Packer::new();
    for i in 0..n {
        p.put_i(-i64::from(i));
        p.put_f32(f32::from(u16::try_from(i % 1000).unwrap()) * 0.5);
        p.put_bytes(&[1, 2, 3, 4, 5, 6, 7, 8]);
        p.put_tagged(&i);
        p.put_str("shared");
    }
    p.finish()
}

fn unpack_scalars(bytes: &[u8], n: u32) -> i64 {
    let mut u = Unpacker::open(bytes).unwrap();
    let mut sum = 0;
    for _ in 0..n {
        sum += u.int().unwrap();
        black_box(u.f32().unwrap());
        sum += i64::try_from(u.bytes().unwrap().len()).unwrap();
        sum += i64::from(u.tagged::<u32>().unwrap());
        sum += i64::try_from(u.shared_str().unwrap().len()).unwrap();
    }
    sum
}

/// The scalar `put_*` and reads, a forked reader, `f32` lists, and a frame read by its shape.
fn scalars(c: &mut Criterion) {
    const N: u32 = 1000;
    let bytes = pack_scalars(N);
    let mut g = group(c, "frame/scalars");
    g.throughput(Throughput::Elements(u64::from(N)));
    g.bench_function("pack_signed_f32_bytes_tagged_str", |b| {
        b.iter(|| pack_scalars(black_box(N)));
    });
    g.bench_function("unpack_signed_f32_bytes_tagged_shared_str", |b| {
        b.iter(|| unpack_scalars(black_box(&bytes), N));
    });
    g.bench_function("fork_and_position", |b| {
        let u = Unpacker::open(&bytes).unwrap();
        b.iter(|| {
            let mut forked = u.fork(black_box(0));
            (forked.position(), forked.int())
        });
    });
    let singles = packed(|p| p.put_f32s(&[0.5, 1.5, 2.5, 3.5]));
    g.bench_function("f32s_four", |b| {
        b.iter(|| Unpacker::open(black_box(&singles)).unwrap().f32s().unwrap());
    });
    let docs_frame = frame::to_frame(&docs(30));
    let shape = Docs::shape();
    g.bench_function("unpack_shaped_docs_medium", |b| {
        b.iter(|| {
            let mut u = Unpacker::open(black_box(&docs_frame)).unwrap();
            frame::unpack_shaped(&mut u, &shape).unwrap()
        });
    });
    g.finish();
}

fn bench(c: &mut Criterion) {
    each(&mut Whole(c), &Size::ALL);
    views(c);
    steps(c);
    numbers_and_floats(c);
    scalars(c);
}

criterion_group! {
    name = benches;
    config = quick();
    targets = bench
}
criterion_main!(benches);
