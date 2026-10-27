//! Running one path over every sample, with a throughput that makes the
//! samples comparable.

use std::hint::black_box;
use std::io;
use std::time::Duration;

use criterion::measurement::WallTime;
use criterion::{BenchmarkGroup, Criterion, Throughput};

use super::sample::Sample;

/// The configuration every bench file uses: short runs, since a file has
/// hundreds of them. Longer ones are asked for on the command line
/// (`--measurement-time 5 --sample-size 100`).
pub(crate) fn quick() -> Criterion {
    Criterion::default()
        .sample_size(20)
        .warm_up_time(Duration::from_millis(200))
        .measurement_time(Duration::from_millis(500))
}

/// A group named `name` whose throughput is set per sample.
pub(crate) fn group<'a>(c: &'a mut Criterion, name: &str) -> BenchmarkGroup<'a, WallTime> {
    c.benchmark_group(name)
}

/// Benchmarks `f` on each of `samples` in a group called `name`; the
/// throughput is the `bytes` of the form the path reads or writes.
pub(crate) fn per_sample<R>(
    c: &mut Criterion,
    name: &str,
    samples: &[Sample],
    bytes: impl Fn(&Sample) -> usize,
    mut f: impl FnMut(&Sample) -> R,
) {
    let mut g = group(c, name);
    for s in samples {
        g.throughput(Throughput::Bytes(bytes(s) as u64));
        g.bench_function(&s.name, |b| b.iter(|| black_box(f(black_box(s)))));
    }
    g.finish();
}

/// A writer that keeps nothing and cannot be optimized away: what a send
/// costs, not what a socket does.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Discard;

impl io::Write for Discard {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(black_box(buf).len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
