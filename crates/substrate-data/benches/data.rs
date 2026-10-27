//! substrate-data against `serde_json` on a task-tracker document (a few
//! thousand tasks with parts, ~1 MB of text): parsing the text, writing
//! it, and the binary form.

#![expect(
    clippy::unwrap_used,
    clippy::print_stderr,
    clippy::cast_precision_loss,
    clippy::format_push_string,
    clippy::shadow_unrelated,
    reason = "a benchmark: fixed inputs that always parse, a generated document, sizes printed"
)]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};

/// Tasks in the generated document.
const TASKS: usize = 4000;

/// A tracker's tasks as an editor would save them: every task with an
/// estimate, most with an attached file and owner, some with a review;
/// ids and names repeat.
fn tasks() -> String {
    let mut out = String::from("{\n  \"name\": \"bench\",\n  \"tasks\": [\n");
    for i in 0..TASKS {
        let f = i as f64;
        out.push_str(&format!(
            "    {{ \"id\": {i}, \"name\": \"task_{i}\", \"parts\": [\n      {{ \"type\": \"estimate\", \"range\": [{:.3}, {:.3}, {:.3}], \"weights\": [1.0, 0.0, 0.0, 0.0] }}",
            f * 0.5,
            (f * 0.25).sin(),
            -f
        ));
        if i % 4 != 0 {
            out.push_str(&format!(
                ",\n      {{ \"type\": \"attachment\", \"file\": \"@file:images/photo_{}.png\", \"owner\": \"@user:alice\" }}",
                i % 16
            ));
        }
        if i % 3 == 0 {
            out.push_str(",\n      { \"type\": \"review\", \"status\": \"open\", \"hours\": 2.5 }");
        }
        out.push_str(if i + 1 < TASKS {
            "\n    ] },\n"
        } else {
            "\n    ] }\n"
        });
    }
    out.push_str("  ]\n}\n");
    out
}

fn bench(c: &mut Criterion) {
    let book = tasks();
    let doc_text = book.as_str();
    let value = substrate_data::text::parse_value(doc_text).unwrap();
    let bytes = substrate_data::binary::encode(&value);
    let json: serde_json::Value = serde_json::from_str(doc_text).unwrap();
    eprintln!(
        "text {} bytes, binary {} bytes",
        doc_text.len(),
        bytes.len()
    );

    let mut g = c.benchmark_group("parse");
    g.bench_function("serde_json", |b| {
        b.iter(|| serde_json::from_str::<serde_json::Value>(black_box(doc_text)).unwrap());
    });
    g.bench_function("substrate_data text", |b| {
        b.iter(|| substrate_data::text::parse_value(black_box(doc_text)).unwrap());
    });
    g.bench_function("substrate_data text + spans", |b| {
        b.iter(|| substrate_data::text::parse(black_box(doc_text)).unwrap());
    });
    g.bench_function("substrate_data binary", |b| {
        b.iter(|| substrate_data::binary::decode(black_box(&bytes)).unwrap());
    });
    g.finish();

    let mut g = c.benchmark_group("write");
    g.bench_function("serde_json", |b| {
        b.iter(|| serde_json::to_string(black_box(&json)).unwrap());
    });
    g.bench_function("substrate_data text", |b| {
        b.iter(|| substrate_data::text::to_string(black_box(&value)));
    });
    g.bench_function("substrate_data binary", |b| {
        b.iter(|| substrate_data::binary::encode(black_box(&value)));
    });
    g.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
