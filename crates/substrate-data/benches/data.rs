//! substrate-data against serde_json on a scene-like document (a few
//! thousand entities with components, ~1 MB of text): parsing the text,
//! writing it, and the binary form.

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

/// Entities in the generated scene.
const ENTITIES: usize = 4000;

/// A scene as an editor would save it: every entity with a transform, most
/// with a mesh and material, some with a body; ids and names repeat.
fn scene() -> String {
    let mut out = String::from("{\n  \"name\": \"bench\",\n  \"entities\": [\n");
    for i in 0..ENTITIES {
        let f = i as f64;
        out.push_str(&format!(
            "    {{ \"id\": {i}, \"name\": \"node_{i}\", \"components\": [\n      {{ \"type\": \"transform\", \"position\": [{:.3}, {:.3}, {:.3}], \"rotation\": [1.0, 0.0, 0.0, 0.0] }}",
            f * 0.5,
            (f * 0.25).sin(),
            -f
        ));
        if i % 4 != 0 {
            out.push_str(&format!(
                ",\n      {{ \"type\": \"renderable\", \"mesh\": \"@mesh:props/crate_{}.mesh\", \"material\": \"@material:wood\" }}",
                i % 16
            ));
        }
        if i % 3 == 0 {
            out.push_str(",\n      { \"type\": \"body\", \"motion\": \"dynamic\", \"mass\": 2.5 }");
        }
        out.push_str(if i + 1 < ENTITIES {
            "\n    ] },\n"
        } else {
            "\n    ] }\n"
        });
    }
    out.push_str("  ]\n}\n");
    out
}

fn bench(c: &mut Criterion) {
    let book = scene();
    let scene_text = book.as_str();
    let value = substrate_data::text::parse_value(scene_text).unwrap();
    let bytes = substrate_data::binary::encode(&value);
    let json: serde_json::Value = serde_json::from_str(scene_text).unwrap();
    eprintln!(
        "text {} bytes, binary {} bytes",
        scene_text.len(),
        bytes.len()
    );

    let mut g = c.benchmark_group("parse");
    g.bench_function("serde_json", |b| {
        b.iter(|| serde_json::from_str::<serde_json::Value>(black_box(scene_text)).unwrap())
    });
    g.bench_function("substrate_data text", |b| {
        b.iter(|| substrate_data::text::parse_value(black_box(scene_text)).unwrap())
    });
    g.bench_function("substrate_data text + spans", |b| {
        b.iter(|| substrate_data::text::parse(black_box(scene_text)).unwrap())
    });
    g.bench_function("substrate_data binary", |b| {
        b.iter(|| substrate_data::binary::decode(black_box(&bytes)).unwrap())
    });
    g.finish();

    let mut g = c.benchmark_group("write");
    g.bench_function("serde_json", |b| {
        b.iter(|| serde_json::to_string(black_box(&json)).unwrap())
    });
    g.bench_function("substrate_data text", |b| {
        b.iter(|| substrate_data::text::to_string(black_box(&value)))
    });
    g.bench_function("substrate_data binary", |b| {
        b.iter(|| substrate_data::binary::encode(black_box(&value)))
    });
    g.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
