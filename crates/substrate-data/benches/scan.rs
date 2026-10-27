//! Scanning text and validating bytes: UTF-8 validation (the standard
//! library's, `simdutf8`, and the crate's own path, which `simd-utf8`
//! switches), and the byte searches (`memchr`) the text parser's scans are
//! built on, over documents that are mostly one kind of thing. Throughput
//! is the bytes scanned.
//!
//! Run it twice to see the feature: `--features simd-utf8` and without.

#![expect(clippy::unwrap_used, reason = "a benchmark: the inputs are valid")]

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use substrate_data::{source, text};

#[expect(dead_code, reason = "the corpus is shared; this bench uses part of it")]
mod corpus;

use corpus::each::{group, quick};
use corpus::lcg::Lcg;

/// A text of long strings: `n` bytes of `unit` repeated.
fn repeated(unit: &str, n: usize) -> String {
    let all = unit.repeat(n / unit.len() + 1);
    all[..all.floor_char_boundary(n)].to_owned()
}

fn utf8(c: &mut Criterion) {
    let n = 500_000;
    let ascii = repeated("The quick brown fox jumps over the lazy dog. ", n);
    let mixed = repeated("naïve café — 日本語のテキスト 😀 Привет. ", n);
    let mut broken = ascii.clone().into_bytes();
    *broken.last_mut().unwrap() = 0xff;
    let mut g = group(c, "utf8");
    for (name, bytes) in [
        ("ascii_500k", ascii.as_bytes()),
        ("mixed_500k", mixed.as_bytes()),
        ("invalid_at_end_500k", &broken),
    ] {
        g.throughput(Throughput::Bytes(bytes.len() as u64));
        g.bench_function(format!("std/{name}"), |b| {
            b.iter(|| std::str::from_utf8(black_box(bytes)).is_ok());
        });
        g.bench_function(format!("simdutf8/{name}"), |b| {
            b.iter(|| simdutf8::basic::from_utf8(black_box(bytes)).is_ok());
        });
        // The crate's own path: `simd-utf8` decides which validator it runs.
        g.bench_function(format!("crate_source_text/{name}"), |b| {
            b.iter(|| source::text(black_box(bytes)).is_ok());
        });
    }
    g.finish();
}

/// Documents that are mostly one thing the scanner searches for.
struct Scanned {
    plain: String,
    escaped: String,
    line_comments: String,
    block_comments: String,
    blank: String,
}

impl Scanned {
    fn new() -> Self {
        let mut rng = Lcg::new(30);
        let mut words = |count: usize| (0..count).map(|_| rng.word()).collect::<Vec<_>>().join(" ");
        Self {
            plain: format!("{{\"k\": \"{}\"}}", words(80_000)),
            escaped: format!("{{\"k\": \"{}\"}}", words(80_000).replace(' ', "\\n ")),
            line_comments: format!(
                "{{\n{}\"k\": 1}}",
                "// a line comment of some length\n".repeat(15_000)
            ),
            block_comments: format!(
                "{{\n{}\"k\": 1}}",
                "/* a block comment of some length */\n".repeat(13_000)
            ),
            blank: format!("{{{}\"k\": 1}}", " \n\t ".repeat(125_000)),
        }
    }
}

fn scans(c: &mut Criterion) {
    let d = Scanned::new();
    let mut g = group(c, "scan");
    for (name, doc) in [
        ("string_no_escapes", &d.plain),
        ("string_escapes", &d.escaped),
        ("line_comments", &d.line_comments),
        ("block_comments", &d.block_comments),
        ("whitespace", &d.blank),
    ] {
        g.throughput(Throughput::Bytes(doc.len() as u64));
        g.bench_function(format!("parse_value/{name}"), |b| {
            b.iter(|| text::parse_value(black_box(doc)).unwrap());
        });
    }
    // The searches themselves: what the parser does over each document, at memchr's speed.
    g.throughput(Throughput::Bytes(d.plain.len() as u64));
    g.bench_function("memchr2_to_closing_quote", |b| {
        b.iter(|| {
            memchr::memchr2(
                b'"',
                b'\\',
                black_box(d.plain.as_bytes()).get(8..).unwrap_or_default(),
            )
        });
    });
    g.throughput(Throughput::Bytes(d.block_comments.len() as u64));
    g.bench_function("memmem_every_block_end", |b| {
        b.iter(|| memchr::memmem::find_iter(black_box(d.block_comments.as_bytes()), b"*/").count());
    });
    g.throughput(Throughput::Bytes(d.line_comments.len() as u64));
    g.bench_function("memchr_every_newline", |b| {
        b.iter(|| memchr::memchr_iter(b'\n', black_box(d.line_comments.as_bytes())).count());
    });
    g.finish();
}

fn bench(c: &mut Criterion) {
    utf8(c);
    scans(c);
}

criterion_group! {
    name = benches;
    config = quick();
    targets = bench
}
criterion_main!(benches);
