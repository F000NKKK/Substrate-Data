//! This crate against `serde_json`, `ciborium` (CBOR), `rmp-serde`
//! (`MessagePack`), `postcard` and `bincode` on the same corpus: typed
//! documents encoded and decoded in every format, and value trees against
//! `serde_json`'s. The sizes are in `cargo run --release --example sizes`.
//!
//! Throughput of every format is the document's JSON length, so the
//! rates compare as "the same data per second"; the time per operation is
//! the number to read.

#![expect(
    clippy::unwrap_used,
    reason = "a benchmark: the corpus always round-trips"
)]

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use substrate_data::{binary, text};

#[expect(dead_code, reason = "the corpus is shared; this bench uses part of it")]
mod corpus;
#[path = "compare/dynamic.rs"]
mod dynamic;

use corpus::each::{group, quick};
use corpus::formats::Fmt;
use corpus::sample::{Size, samples};
use corpus::typed::{Typed, Visit, each};

/// Every typed document in every format.
struct Typed9<'a>(&'a mut Criterion);

impl Visit for Typed9<'_> {
    fn visit<T: Typed>(&mut self, name: &str, value: &T) {
        let logical = Fmt::Json.encode(value).len() as u64;
        let mut enc = group(self.0, "compare/typed_encode");
        enc.throughput(Throughput::Bytes(logical));
        for f in Fmt::ALL {
            enc.bench_function(format!("{name}/{f}"), |b| {
                b.iter(|| f.encode(black_box(value)));
            });
        }
        enc.finish();
        let mut dec = group(self.0, "compare/typed_decode");
        dec.throughput(Throughput::Bytes(logical));
        for f in Fmt::ALL {
            let bytes = f.encode(value);
            corpus::formats::check(f, value, &bytes, name);
            dec.bench_function(format!("{name}/{f}"), |b| {
                b.iter(|| f.decode::<T>(black_box(&bytes)).unwrap());
            });
        }
        dec.finish();
    }
}

/// Value trees: this crate's two forms against the serde ones through `serde_json::Value`.
fn dynamic(c: &mut Criterion) {
    for s in samples() {
        let tree = dynamic::tree(&s.value);
        let json = serde_json::to_vec(&tree).unwrap();
        let mut cbor = Vec::new();
        ciborium::into_writer(&tree, &mut cbor).unwrap();
        let msgpack = rmp_serde::to_vec_named(&tree).unwrap();
        let mut g = group(c, "compare/dynamic_encode");
        g.throughput(Throughput::Bytes(json.len() as u64));
        g.bench_function(format!("{}/sd_binary", s.name), |b| {
            b.iter(|| binary::encode(black_box(&s.value)));
        });
        g.bench_function(format!("{}/sd_text", s.name), |b| {
            b.iter(|| text::to_string(black_box(&s.value)));
        });
        g.bench_function(format!("{}/json", s.name), |b| {
            b.iter(|| serde_json::to_vec(black_box(&tree)).unwrap());
        });
        g.bench_function(format!("{}/cbor", s.name), |b| {
            b.iter(|| {
                let mut out = Vec::new();
                ciborium::into_writer(black_box(&tree), &mut out).unwrap();
                out
            });
        });
        g.bench_function(format!("{}/msgpack", s.name), |b| {
            b.iter(|| rmp_serde::to_vec_named(black_box(&tree)).unwrap());
        });
        g.finish();
        let mut dec = group(c, "compare/dynamic_decode");
        dec.throughput(Throughput::Bytes(json.len() as u64));
        dec.bench_function(format!("{}/sd_binary", s.name), |b| {
            b.iter(|| binary::decode(black_box(&s.bytes)).unwrap());
        });
        dec.bench_function(format!("{}/sd_text", s.name), |b| {
            b.iter(|| text::parse_value(black_box(&s.text)).unwrap());
        });
        dec.bench_function(format!("{}/json", s.name), |b| {
            b.iter(|| serde_json::from_slice::<serde_json::Value>(black_box(&json)).unwrap());
        });
        dec.bench_function(format!("{}/cbor", s.name), |b| {
            b.iter(|| ciborium::from_reader::<serde_json::Value, _>(black_box(&cbor[..])).unwrap());
        });
        dec.bench_function(format!("{}/msgpack", s.name), |b| {
            b.iter(|| rmp_serde::from_slice::<serde_json::Value>(black_box(&msgpack)).unwrap());
        });
        dec.finish();
    }
}

fn bench(c: &mut Criterion) {
    each(&mut Typed9(c), &Size::ALL);
    dynamic(c);
}

criterion_group! {
    name = benches;
    config = quick();
    targets = bench
}
criterion_main!(benches);
