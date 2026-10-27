//! Bit-packing of lists of numbers: every instruction set this processor has,
//! group by group, and the whole `pack`/`unpack` through the active one.

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use substrate_data::frame::lanes::{self, Backend, GROUP};

/// Widths a list of ids, counters, indices and flags tends to have.
const WIDTHS: [u32; 6] = [1, 4, 8, 13, 17, 32];
/// Groups per run: 64 × 256 numbers, 16 K numbers.
const GROUPS: usize = 64;

fn numbers(width: u32) -> Vec<[u32; GROUP]> {
    let mask = u32::MAX >> (u32::BITS - width);
    let mut x = 0x9E37_79B9_u32;
    (0..GROUPS)
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
    let mut pack = c.benchmark_group("lanes/pack");
    pack.throughput(Throughput::Elements((GROUPS * GROUP) as u64));
    for width in WIDTHS {
        let src = numbers(width);
        for backend in Backend::available() {
            let mut out = vec![0; GROUPS * lanes::group_bytes(width)];
            pack.bench_with_input(
                BenchmarkId::new(backend.to_string(), width),
                &width,
                |b, &w| b.iter(|| backend.pack_groups(black_box(&src), w, &mut out)),
            );
        }
    }
    pack.finish();

    let mut unpack = c.benchmark_group("lanes/unpack");
    unpack.throughput(Throughput::Elements((GROUPS * GROUP) as u64));
    for width in WIDTHS {
        let src = numbers(width);
        let mut bytes = vec![0; GROUPS * lanes::group_bytes(width)];
        Backend::Portable.pack_groups(&src, width, &mut bytes);
        for backend in Backend::available() {
            let mut out = vec![[0; GROUP]; GROUPS];
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
    let n = GROUPS * GROUP;
    let mut g = c.benchmark_group("lanes/whole");
    g.throughput(Throughput::Elements(n as u64));
    for width in WIDTHS {
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

criterion_group!(benches, groups, whole);
criterion_main!(benches);
