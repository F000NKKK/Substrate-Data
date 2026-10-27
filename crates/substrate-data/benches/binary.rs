//! The binary form: whole values (encode, encode into a reused buffer,
//! decode, decode borrowed), malformed input, and the step-by-step
//! `Writer` and `Decoder` a schema drives. One group per path, every sample
//! of the corpus in each; throughput is the bytes of the binary form.

#![expect(clippy::unwrap_used, reason = "a benchmark: the corpus always decodes")]

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use substrate_data::Bump;
use substrate_data::binary::{self, Decoder, Writer};

#[expect(dead_code, reason = "the corpus is shared; this bench uses part of it")]
mod corpus;

use corpus::each::{group, per_sample, quick};
use corpus::sample::{Sample, samples};
use corpus::typed::{DocRecord, docs};

fn whole(c: &mut Criterion, all: &[Sample]) {
    let len = |s: &Sample| s.bytes.len();
    per_sample(c, "binary/encode", all, len, |s| binary::encode(&s.value));
    let mut out = Vec::new();
    per_sample(c, "binary/encode_into_reused", all, len, |s| {
        out.clear();
        binary::encode_into(&s.value, &mut out);
        out.len()
    });
    per_sample(c, "binary/writer_value", all, len, |s| {
        let mut w = Writer::new();
        w.value(&s.value);
        w.finish()
    });
    per_sample(c, "binary/decode", all, len, |s| {
        binary::decode(&s.bytes).unwrap()
    });
    per_sample(c, "binary/decoder_value", all, len, |s| {
        Decoder::open(&s.bytes).unwrap().value().unwrap()
    });
    per_sample(c, "binary/decode_ref_fresh_arena", all, len, |s| {
        let arena = Bump::new();
        binary::decode_ref(&s.bytes, &arena).unwrap().to_value()
    });
}

fn borrowed(c: &mut Criterion, all: &[Sample]) {
    let len = |s: &Sample| s.bytes.len();
    let mut arena = Bump::new();
    per_sample(c, "binary/decode_ref_reused_arena", all, len, |s| {
        arena.reset();
        binary::decode_ref(&s.bytes, &arena).unwrap().is_null()
    });
    per_sample(c, "binary/decoder_value_ref", all, len, |s| {
        arena.reset();
        let mut d = Decoder::open(&s.bytes).unwrap();
        d.value_ref(&arena).unwrap().is_null()
    });
    per_sample(c, "binary/value_borrow", all, len, |s| {
        arena.reset();
        s.value.borrow(&arena).is_null()
    });
    per_sample(c, "binary/at_end_after_open", all, len, |s| {
        Decoder::open(&s.bytes).unwrap().at_end()
    });
}

/// Input that is cut, or has a wrong tag: the error paths.
fn malformed(c: &mut Criterion, all: &[Sample]) {
    let cut: Vec<(String, Vec<u8>)> = all
        .iter()
        .map(|s| (s.name.clone(), s.bytes[..s.bytes.len() * 3 / 4].to_vec()))
        .collect();
    let mut g = group(c, "binary/decode_truncated");
    for (name, bytes) in &cut {
        g.throughput(Throughput::Bytes(bytes.len() as u64));
        g.bench_function(name, |b| {
            b.iter(|| binary::decode(black_box(bytes)).is_err());
        });
    }
    g.finish();
    let bad = [
        b"SDAT\x01\xff".as_slice(),
        b"nope".as_slice(),
        b"SDAT\x09\x00".as_slice(),
    ];
    let mut errors = group(c, "binary/decode_error_display");
    for (i, bytes) in bad.iter().enumerate() {
        errors.bench_function(format!("bad_{i}"), |b| {
            b.iter(|| binary::decode(black_box(bytes)).unwrap_err().to_string());
        });
    }
    errors.finish();
}

/// What a schema's `encode` does: a list of maps, field by field.
fn write_records(items: &[DocRecord]) -> Vec<u8> {
    let mut w = Writer::new();
    w.list(items.len());
    for d in items {
        w.map(4)
            .key("id")
            .int(i64::from(d.id))
            .key("title")
            .str(&d.title)
            .key("author")
            .str(&d.author)
            .key("rating")
            .float(d.rating);
    }
    w.finish()
}

fn read_records(bytes: &[u8]) -> usize {
    let mut d = Decoder::open(bytes).unwrap();
    let n = d.list().unwrap();
    let mut chars = 0;
    for _ in 0..n {
        d.map().unwrap();
        for _ in 0..4 {
            let key = d.key().unwrap();
            chars += match key {
                "id" => usize::try_from(d.int().unwrap()).unwrap(),
                "rating" => {
                    black_box(d.float().unwrap());
                    0
                }
                _ => d.str().unwrap().len(),
            };
        }
        d.leave();
    }
    chars
}

fn steps(c: &mut Criterion) {
    for (label, n) in [("small", 4), ("medium", 100), ("large", 10_000)] {
        let items = docs(n).docs;
        let bytes = write_records(&items);
        let mut g = group(c, "binary/steps");
        g.throughput(Throughput::Bytes(bytes.len() as u64));
        g.bench_function(format!("writer_records_{label}"), |b| {
            b.iter(|| write_records(black_box(&items)));
        });
        g.bench_function(format!("decoder_records_{label}"), |b| {
            b.iter(|| read_records(black_box(&bytes)));
        });
        g.finish();
    }
}

/// What `step` writes into a new writer.
fn written(step: impl FnOnce(&mut Writer) -> &mut Writer) -> Vec<u8> {
    let mut w = Writer::new();
    step(&mut w);
    w.finish()
}

/// Lists of numbers, written and read as the packed lists they are.
fn numbers(c: &mut Criterion) {
    let n = 10_000usize;
    let ints: Vec<i64> = (0..10_000i64).map(|i| i * 7 % 1000 - 500).collect();
    let wide: Vec<f64> = (0..10_000u32).map(|i| f64::from(i) * 0.1 + 1e-9).collect();
    let narrow: Vec<f32> = (0..10_000u16).map(|i| f32::from(i) * 0.5).collect();
    let mut g = group(c, "binary/packed_lists");
    g.throughput(Throughput::Elements(n as u64));
    g.bench_function("write_ints", |b| {
        b.iter(|| written(|w| w.ints(ints.iter().copied())));
    });
    g.bench_function("write_floats", |b| {
        b.iter(|| written(|w| w.floats(black_box(&wide))));
    });
    g.bench_function("write_floats32", |b| {
        b.iter(|| written(|w| w.floats32(black_box(&narrow))));
    });
    let int_bytes = written(|w| w.ints(ints.iter().copied()));
    let wide_bytes = written(|w| w.floats(&wide));
    let narrow_bytes = written(|w| w.floats32(&narrow));
    g.bench_function("read_ints", |b| {
        b.iter(|| {
            let mut d = Decoder::open(black_box(&int_bytes)).unwrap();
            let count = d.ints().unwrap();
            (0..count).map(|_| d.packed_int().unwrap()).sum::<i64>()
        });
    });
    g.bench_function("read_floats", |b| {
        b.iter(|| {
            Decoder::open(black_box(&wide_bytes))
                .unwrap()
                .floats()
                .unwrap()
        });
    });
    g.bench_function("read_floats32", |b| {
        b.iter(|| {
            Decoder::open(black_box(&narrow_bytes))
                .unwrap()
                .floats32()
                .unwrap()
        });
    });
    g.finish();
}

/// A thousand of every scalar, written with the step-by-step `Writer`.
fn write_scalars(n: usize) -> Vec<u8> {
    let mut w = Writer::new();
    w.list(n * 7);
    for i in 0..n {
        w.null()
            .bool(i % 2 == 0)
            .int(-i64::try_from(i).unwrap())
            .float32(0.5)
            .float(0.1)
            .str("shared")
            .bytes(&[1, 2, 3, 4]);
    }
    w.finish()
}

/// The same read back: `null`, `bool`, `int`, `float`, `str`, `bytes`, and `skip` past a value.
fn read_scalars(bytes: &[u8], n: usize) -> usize {
    let mut d = Decoder::open(bytes).unwrap();
    d.list().unwrap();
    let mut seen = 0;
    for i in 0..n {
        seen += usize::from(d.null());
        seen += usize::from(d.bool().unwrap());
        seen += usize::try_from(d.int().unwrap().unsigned_abs()).unwrap();
        seen += usize::from(black_box(d.float().unwrap()) > 0.0);
        seen += usize::from(black_box(d.float().unwrap()) > 0.0);
        if i % 2 == 0 {
            d.skip().unwrap();
        } else {
            seen += d.str().unwrap().len();
        }
        seen += d.bytes().unwrap().len();
    }
    seen + usize::from(d.at_end())
}

fn scalars(c: &mut Criterion) {
    let n = 1000;
    let bytes = write_scalars(n);
    let mut g = group(c, "binary/scalars");
    g.throughput(Throughput::Bytes(bytes.len() as u64));
    g.bench_function("writer_null_bool_int_float32_str_bytes", |b| {
        b.iter(|| write_scalars(black_box(n)));
    });
    g.bench_function("decoder_null_bool_int_float_str_skip_bytes", |b| {
        b.iter(|| read_scalars(black_box(&bytes), n));
    });
    g.finish();
}

fn bench(c: &mut Criterion) {
    let all = samples();
    whole(c, &all);
    borrowed(c, &all);
    malformed(c, &all);
    steps(c);
    scalars(c);
    numbers(c);
}

criterion_group! {
    name = benches;
    config = quick();
    targets = bench
}
criterion_main!(benches);
