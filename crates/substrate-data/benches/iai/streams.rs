//! Streams: decoding a binary document that holds one (every item is
//! checked then) and reading its items.

#![expect(
    unused_qualifications,
    reason = "the harness macros qualify the paths of what they expand"
)]
#![expect(clippy::unwrap_used, reason = "a benchmark: the stream always reads")]

use std::hint::black_box;

use iai_callgrind::{library_benchmark, library_benchmark_group};
use substrate_data::{Schema, Stream, read_bytes, to_bytes};

/// A record with a string, as a table row.
#[derive(Schema)]
struct Row {
    label: String,
    cells: Vec<u16>,
}

#[derive(Schema)]
struct Rows {
    rows: Stream<Row>,
}

#[derive(Schema)]
struct Ints {
    values: Stream<u32>,
}

fn rows_bytes() -> &'static [u8] {
    let rows = (0..1000u16)
        .map(|i| Row {
            label: format!("row {i}"),
            cells: vec![i, i + 1],
        })
        .collect();
    Box::leak(to_bytes(&Rows { rows }).into_boxed_slice())
}

fn ints_bytes() -> &'static [u8] {
    let values = (0..10_000u32).collect();
    Box::leak(to_bytes(&Ints { values }).into_boxed_slice())
}

#[library_benchmark]
#[bench::rows_1000(rows_bytes())]
fn rows_read_bytes(bytes: &'static [u8]) -> Rows {
    black_box(read_bytes::<Rows>(bytes).unwrap())
}

#[library_benchmark]
#[bench::ints_10000(ints_bytes())]
fn ints_read_bytes(bytes: &'static [u8]) -> Ints {
    black_box(read_bytes::<Ints>(bytes).unwrap())
}

#[library_benchmark]
#[bench::rows_1000(read_bytes::<Rows>(rows_bytes()).unwrap())]
fn rows_iterate(rows: Rows) -> usize {
    black_box(rows.rows.iter().count())
}

library_benchmark_group!(name = stream_forms; benchmarks = rows_read_bytes, ints_read_bytes, rows_iterate);
