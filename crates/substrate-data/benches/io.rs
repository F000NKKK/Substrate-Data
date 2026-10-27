//! Frames on a wire: a message written with its length in front and read
//! back, a frame at a time and as a stream of many (`write_frame`,
//! `FrameWriter`, `read_frame`, `FrameReader`). Writers go to a sink that
//! keeps nothing; readers read from memory. Throughput is the bytes of the
//! message.

#![expect(
    clippy::unwrap_used,
    reason = "a benchmark: the corpus always reads back"
)]

use std::hint::black_box;
use std::io::{Cursor, Write};

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use substrate_data::frame;
use substrate_data::io::{
    FrameReader, FrameWriter, read_frame, read_frame_bytes, write_frame, write_frame_bytes,
};

#[expect(dead_code, reason = "the corpus is shared; this bench uses part of it")]
mod corpus;

use corpus::each::{Discard, group, quick};
use corpus::typed::{Docs, Users, docs, users};

/// The same bytes to a sink, a frame at a time and as a stream of many.
fn io(c: &mut Criterion) {
    let mut g = group(c, "io");
    for (label, n) in [("small", 1), ("medium", 30), ("large", 2900)] {
        let message = docs(n);
        let body = substrate_data::to_bytes(&message);
        g.throughput(Throughput::Bytes(body.len() as u64));
        g.bench_function(format!("write_frame/{label}"), |b| {
            b.iter(|| write_frame(&mut Discard, black_box(&message)).unwrap());
        });
        g.bench_function(format!("write_frame_bytes/{label}"), |b| {
            b.iter(|| write_frame_bytes(&mut Discard, black_box(&body)).unwrap());
        });
        let mut sender = FrameWriter::new();
        g.bench_function(format!("FrameWriter_send/{label}"), |b| {
            b.iter(|| sender.send(&mut Discard, black_box(&message)).unwrap());
        });
        g.bench_function(format!("FrameWriter_send_packed/{label}"), |b| {
            b.iter(|| {
                sender
                    .send_packed(&mut Discard, black_box(&message))
                    .unwrap();
            });
        });
        read_side(&mut g, label, &message);
    }
    g.finish();
}

fn read_side(
    g: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    label: &str,
    message: &Docs,
) {
    let mut wire = Vec::new();
    write_frame(&mut wire, message).unwrap();
    let mut packed = Vec::new();
    FrameWriter::new()
        .send_packed(&mut packed, message)
        .unwrap();
    g.bench_function(format!("read_frame_bytes/{label}"), |b| {
        b.iter(|| read_frame_bytes(&mut Cursor::new(black_box(&wire))).unwrap());
    });
    g.bench_function(format!("read_frame/{label}"), |b| {
        b.iter(|| read_frame::<Docs>(&mut Cursor::new(black_box(&wire))).unwrap());
    });
    let mut reader = FrameReader::new();
    g.bench_function(format!("FrameReader_read_bytes/{label}"), |b| {
        b.iter(|| {
            reader
                .read_bytes(&mut Cursor::new(black_box(&wire)))
                .unwrap()
                .map(<[u8]>::len)
        });
    });
    g.bench_function(format!("FrameReader_read/{label}"), |b| {
        b.iter(|| {
            reader
                .read::<Docs>(&mut Cursor::new(black_box(&wire)))
                .unwrap()
        });
    });
    g.bench_function(format!("FrameReader_read_bytes_unpack/{label}"), |b| {
        b.iter(|| {
            let body = reader
                .read_bytes(&mut Cursor::new(black_box(&packed)))
                .unwrap()
                .unwrap();
            frame::read_frame::<Docs>(body).unwrap()
        });
    });
}

/// A thousand small messages on one stream: what a connection carries.
fn many(c: &mut Criterion) {
    let message = users(1);
    let mut wire = Vec::new();
    let mut sender = FrameWriter::new();
    for _ in 0..1000 {
        sender.send(&mut wire, &message).unwrap();
    }
    let mut g = group(c, "io/stream_1000");
    g.throughput(Throughput::Elements(1000));
    g.bench_function("send", |b| {
        b.iter(|| {
            let mut out = Vec::with_capacity(wire.len());
            for _ in 0..1000 {
                sender.send(&mut out, black_box(&message)).unwrap();
            }
            out.flush().unwrap();
            out
        });
    });
    let mut reader = FrameReader::new();
    g.bench_function("read", |b| {
        b.iter(|| {
            let mut input = Cursor::new(black_box(&wire));
            let mut n = 0;
            while reader.read::<Users>(&mut input).unwrap().is_some() {
                n += 1;
            }
            n
        });
    });
    g.finish();
}

fn bench(c: &mut Criterion) {
    io(c);
    many(c);
}

criterion_group! {
    name = benches;
    config = quick();
    targets = bench
}
criterion_main!(benches);
