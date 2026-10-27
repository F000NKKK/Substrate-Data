//! The async frame paths (`tokio` feature) next to the blocking ones in
//! `io.rs`: the same messages written to a sink and read from memory, driven
//! on a current-thread runtime. The difference is what `.await` adds when the
//! stream is always ready. Throughput is the bytes of the message.

#![expect(
    clippy::unwrap_used,
    reason = "a benchmark: the corpus always reads back"
)]

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use substrate_data::io::asynchronous::{FrameReader, FrameWriter};
use substrate_data::io::write_frame;
use tokio::runtime::Runtime;

#[expect(dead_code, reason = "the corpus is shared; this bench uses part of it")]
mod corpus;

use corpus::each::{group, quick};
use corpus::typed::{Docs, Users, docs, users};

fn runtime() -> Runtime {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
}

/// One message a time: `send` to a sink and `read` from memory.
fn io(c: &mut Criterion) {
    let rt = runtime();
    let mut g = group(c, "io_async");
    for (label, n) in [("small", 1), ("medium", 30), ("large", 2900)] {
        let message = docs(n);
        let body = substrate_data::to_bytes(&message);
        let mut wire = Vec::new();
        write_frame(&mut wire, &message).unwrap();
        g.throughput(Throughput::Bytes(body.len() as u64));
        let mut sender = FrameWriter::new();
        g.bench_function(format!("FrameWriter_send/{label}"), |b| {
            b.iter(|| {
                rt.block_on(sender.send(&mut tokio::io::sink(), black_box(&message)))
                    .unwrap();
            });
        });
        let mut reader = FrameReader::new();
        g.bench_function(format!("FrameReader_read/{label}"), |b| {
            b.iter(|| {
                rt.block_on(reader.read::<Docs>(&mut black_box(wire.as_slice())))
                    .unwrap()
            });
        });
        g.bench_function(format!("FrameReader_read_bytes/{label}"), |b| {
            b.iter(|| {
                rt.block_on(async {
                    reader
                        .read_bytes(&mut black_box(wire.as_slice()))
                        .await
                        .unwrap()
                        .map(<[u8]>::len)
                })
            });
        });
    }
    g.finish();
}

/// A thousand small messages on one stream, one `block_on` for all of them.
fn many(c: &mut Criterion) {
    let rt = runtime();
    let message = users(1);
    let mut wire = Vec::new();
    let mut sender = FrameWriter::new();
    rt.block_on(async {
        for _ in 0..1000 {
            sender.send(&mut wire, &message).await.unwrap();
        }
    });
    let mut g = group(c, "io_async/stream_1000");
    g.throughput(Throughput::Elements(1000));
    g.bench_function("send", |b| {
        b.iter(|| {
            let mut out = Vec::with_capacity(wire.len());
            rt.block_on(async {
                for _ in 0..1000 {
                    sender.send(&mut out, black_box(&message)).await.unwrap();
                }
            });
            out
        });
    });
    let mut reader = FrameReader::new();
    g.bench_function("read", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut input = black_box(wire.as_slice());
                let mut n = 0;
                while reader.read::<Users>(&mut input).await.unwrap().is_some() {
                    n += 1;
                }
                n
            })
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
