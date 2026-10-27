//! Bit-packing of lists of numbers: every instruction set this processor has,
//! group by group, and the whole `pack`/`unpack` through the active one.
//!
//! Groups per run: 64 by default, `LANES_GROUPS=3` for another count.
//!
//! Widths: a representative ten by default; `LANES_WIDTHS=all` for 1..=32,
//! or `LANES_WIDTHS=3,17,24` for a list. Criterion's own filter works too
//! (`cargo bench --bench lanes -- lanes/unpack/avx2/17`).

use std::hint::black_box;
use std::time::Duration;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use substrate_data::frame::lanes::{self, Backend, GROUP};

/// Widths a list of ids, counters, indices and flags tends to have: the
/// byte-aligned ones, and the ones next to them.
const DEFAULT_WIDTHS: [u32; 10] = [1, 2, 4, 7, 8, 12, 16, 17, 24, 32];

/// The widths to run: `LANES_WIDTHS` (`all`, or a comma-separated list), else the default ten.
fn widths() -> Vec<u32> {
    match std::env::var("LANES_WIDTHS") {
        Ok(v) if v == "all" => (1..=32).collect(),
        Ok(v) => v
            .split(',')
            .filter_map(|w| w.trim().parse().ok())
            .filter(|w| (1..=32).contains(w))
            .collect(),
        Err(_) => DEFAULT_WIDTHS.to_vec(),
    }
}
/// Groups per run: `LANES_GROUPS`, else 64 × 256 numbers, 16 K numbers. A
/// short run (1 to 5) shows where a backend hands the tail to a narrower one.
fn group_count() -> usize {
    std::env::var("LANES_GROUPS")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&n| n >= 1)
        .unwrap_or(64)
}

fn numbers(width: u32) -> Vec<[u32; GROUP]> {
    let groups = group_count();
    let mask = u32::MAX >> (u32::BITS - width);
    let mut x = 0x9E37_79B9_u32;
    (0..groups)
        .map(|_| {
            std::array::from_fn(|_| {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                x & mask
            })
        })
        .collect()
}

fn groups(c: &mut Criterion) {
    let count = group_count();
    c.bench_function("lanes/is_available_all", |b| {
        b.iter(|| {
            Backend::available()
                .filter(|backend| black_box(*backend).is_available())
                .count()
        });
    });
    let mut pack = c.benchmark_group("lanes/pack");
    pack.throughput(Throughput::Elements((count * GROUP) as u64));
    for width in widths() {
        let src = numbers(width);
        for backend in Backend::available() {
            let mut out = vec![0; count * lanes::group_bytes(width)];
            pack.bench_with_input(
                BenchmarkId::new(backend.to_string(), width),
                &width,
                |b, &w| b.iter(|| backend.pack_groups(black_box(&src), w, &mut out)),
            );
        }
    }
    pack.finish();

    let mut unpack = c.benchmark_group("lanes/unpack");
    unpack.throughput(Throughput::Elements((count * GROUP) as u64));
    for width in widths() {
        let src = numbers(width);
        let mut bytes = vec![0; count * lanes::group_bytes(width)];
        Backend::Portable.pack_groups(&src, width, &mut bytes);
        for backend in Backend::available() {
            let mut out = vec![[0; GROUP]; count];
            unpack.bench_with_input(
                BenchmarkId::new(backend.to_string(), width),
                &width,
                |b, &w| b.iter(|| backend.unpack_groups(black_box(&bytes), w, &mut out)),
            );
        }
    }
    unpack.finish();
}

/// The whole path a frame takes: an iterator in, bytes out, and back.
fn whole(c: &mut Criterion) {
    let n = group_count() * GROUP;
    let mut g = c.benchmark_group("lanes/whole");
    g.throughput(Throughput::Elements(n as u64));
    for width in widths() {
        let items: Vec<u64> = numbers(width)
            .iter()
            .flatten()
            .map(|&v| u64::from(v))
            .collect();
        let mut packed = Vec::new();
        lanes::pack(&mut packed, n, width, items.iter().copied());
        g.bench_with_input(BenchmarkId::new("pack", width), &width, |b, &w| {
            b.iter(|| {
                let mut out = Vec::with_capacity(packed.len());
                lanes::pack(&mut out, n, w, items.iter().copied());
                black_box(out)
            });
        });
        g.bench_with_input(BenchmarkId::new("unpack", width), &width, |b, &w| {
            b.iter(|| {
                let mut sum = 0u64;
                lanes::unpack(black_box(&packed), n, w, |v| sum = sum.wrapping_add(v));
                black_box(sum)
            });
        });
    }
    g.finish();
}

/// The small calls: a group's size, a list's size, one element out of a packed list.
fn small(c: &mut Criterion) {
    let mut g = c.benchmark_group("lanes/small");
    g.bench_function("group_bytes_packed_bytes", |b| {
        b.iter(|| {
            (1..=32u32)
                .map(|w| {
                    lanes::group_bytes(black_box(w)) + lanes::packed_bytes(1000, w).unwrap_or(0)
                })
                .sum::<usize>()
        });
    });
    for width in widths() {
        let n = group_count() * GROUP;
        let items: Vec<u64> = numbers(width)
            .iter()
            .flatten()
            .map(|&v| u64::from(v))
            .collect();
        let mut packed = Vec::new();
        lanes::pack(&mut packed, n, width, items.iter().copied());
        g.bench_with_input(BenchmarkId::new("get_one", width), &width, |b, &w| {
            b.iter(|| lanes::get(black_box(&packed), n, w, n / 2));
        });
    }
    g.finish();
}

/// Few, short runs: there are hundreds of them. `--measurement-time 3` for steadier ones.
fn quick() -> Criterion {
    Criterion::default()
        .sample_size(20)
        .warm_up_time(Duration::from_millis(200))
        .measurement_time(Duration::from_millis(500))
}

criterion_group! {
    name = benches;
    config = quick();
    targets = groups, whole, small
}
criterion_main!(benches);
