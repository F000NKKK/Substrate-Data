//! Big streams: decoding a binary document that holds a million items, and
//! reading them. Decoding cuts the stream out whole (no item is read);
//! iterating reads each item, and can fail on a bad one.

#![expect(clippy::unwrap_used, reason = "a benchmark: the stream always reads")]
#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "generated numbers, any value will do"
)]

use std::hint::black_box;
use std::time::Duration;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use substrate_data::{Schema, Stream, read_bytes, to_bytes};

const ITEMS: usize = 1_000_000;

/// A table's row: three small fields.
#[derive(Schema, Clone, PartialEq, Debug)]
struct Row {
    id: u32,
    weight: f32,
    label: u8,
}

#[derive(Schema)]
struct Small {
    cells: Stream<u16>,
}

#[derive(Schema)]
struct Floats {
    cells: Stream<f32>,
}

#[derive(Schema)]
struct Rows {
    rows: Stream<Row>,
}

fn small() -> Small {
    Small {
        cells: (0..ITEMS).map(|i| (i % 3000) as u16).collect(),
    }
}

fn floats() -> Floats {
    Floats {
        cells: (0..ITEMS).map(|i| i as f32 * 0.25).collect(),
    }
}

fn rows() -> Rows {
    Rows {
        rows: (0..ITEMS)
            .map(|i| Row {
                id: i as u32,
                weight: (i % 100) as f32 * 0.5,
                label: (i % 7) as u8,
            })
            .collect(),
    }
}

/// Decode of `bytes` (what a reader pays when a message arrives).
fn decode<T: Schema>(c: &mut Criterion, name: &str, bytes: &[u8]) {
    let mut g = c.benchmark_group("stream_decode");
    g.sample_size(30)
        .measurement_time(Duration::from_secs(5))
        .warm_up_time(Duration::from_secs(1))
        .throughput(Throughput::Elements(ITEMS as u64));
    g.bench_function(format!("{name}/decode"), |b| {
        b.iter(|| read_bytes::<T>(black_box(bytes)).unwrap());
    });
    g.finish();
}

fn bench(c: &mut Criterion) {
    let small_bytes = to_bytes(&small());
    decode::<Small>(c, "u16_1m", &small_bytes);
    let floats_bytes = to_bytes(&floats());
    decode::<Floats>(c, "f32_1m", &floats_bytes);
    let rows_bytes = to_bytes(&rows());
    decode::<Rows>(c, "row_1m", &rows_bytes);

    let mut g = c.benchmark_group("stream_iterate");
    g.sample_size(30)
        .measurement_time(Duration::from_secs(5))
        .warm_up_time(Duration::from_secs(1))
        .throughput(Throughput::Elements(ITEMS as u64));
    let s = read_bytes::<Small>(&small_bytes).unwrap();
    g.bench_function("u16_1m/sum", |b| {
        b.iter(|| iterate_sum(black_box(&s.cells)));
    });
    let r = read_bytes::<Rows>(&rows_bytes).unwrap();
    g.bench_function("row_1m/sum", |b| {
        b.iter(|| iterate_rows(black_box(&r.rows)));
    });
    g.finish();
}

/// Every cell, read and added up.
fn iterate_sum(cells: &Stream<u16>) -> u64 {
    cells.iter().map(|c| u64::from(c.unwrap())).sum()
}

/// Every row, read and its id added up.
fn iterate_rows(rows: &Stream<Row>) -> u64 {
    rows.iter().map(|r| u64::from(r.unwrap().id)).sum()
}

criterion_group!(benches, bench);
criterion_main!(benches);
