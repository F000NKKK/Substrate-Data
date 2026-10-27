//! The model: `Map` lookups and edits at 4 to 4096 keys, `Value` and
//! `Path`, borrowed values and the positions of text. One group per area;
//! throughput is elements (lookups) or the bytes of the document.

#![expect(clippy::unwrap_used, reason = "a benchmark: the inputs are valid")]

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use substrate_data::{Bump, LineCol, Map, Path, Ref, Seg, Value, binary, text};

#[expect(dead_code, reason = "the corpus is shared; this bench uses part of it")]
mod corpus;

use corpus::each::{group, quick};
use corpus::sample::{Sample, Size, Topic};

/// Key counts a map is measured at: a record, a small table, a big one, a huge one.
const KEYS: [usize; 4] = [4, 16, 256, 4096];

fn map_of(n: usize) -> Map {
    let mut m = Map::with_capacity(n);
    for i in 0..n {
        m.insert(format!("key_{i}"), Value::Int(i64::try_from(i).unwrap()));
    }
    m
}

/// A map of `n` keys, the keys, and keys it doesn't have.
struct Keyed {
    n: usize,
    map: Map,
    hits: Vec<String>,
    misses: Vec<String>,
}

impl Keyed {
    fn new(n: usize) -> Self {
        Self {
            n,
            map: map_of(n),
            hits: (0..n).map(|i| format!("key_{i}")).collect(),
            misses: (0..n).map(|i| format!("absent_{i}")).collect(),
        }
    }
}

fn maps(c: &mut Criterion) {
    for n in KEYS {
        let k = Keyed::new(n);
        map_reads(c, &k);
        map_writes(c, &k);
    }
}

fn map_reads(c: &mut Criterion, k: &Keyed) {
    let n = k.n;
    let mut g = group(c, "map");
    g.throughput(Throughput::Elements(n as u64));
    g.bench_function(BenchmarkId::new("get_hit_every_key", n), |b| {
        b.iter(|| k.hits.iter().filter(|key| k.map.get(key).is_some()).count());
    });
    g.bench_function(BenchmarkId::new("get_miss", n), |b| {
        b.iter(|| {
            k.misses
                .iter()
                .filter(|key| k.map.get(key).is_none())
                .count()
        });
    });
    g.bench_function(BenchmarkId::new("index_of_value_at", n), |b| {
        b.iter(|| {
            k.hits
                .iter()
                .filter_map(|key| k.map.index_of(key))
                .filter_map(|i| k.map.value_at(i))
                .count()
        });
    });
    g.bench_function(BenchmarkId::new("iter_keys_values", n), |b| {
        b.iter(|| k.map.iter().count() + k.map.keys().count() + k.map.values().count());
    });
    g.bench_function(BenchmarkId::new("clone", n), |b| b.iter(|| k.map.clone()));
    let other = k.map.clone();
    g.bench_function(BenchmarkId::new("eq", n), |b| {
        b.iter(|| k.map == black_box(&other).clone());
    });
    g.finish();
}

fn map_writes(c: &mut Criterion, k: &Keyed) {
    let n = k.n;
    let mut g = group(c, "map");
    g.throughput(Throughput::Elements(n as u64));
    g.bench_function(BenchmarkId::new("get_mut", n), |b| {
        let mut own = k.map.clone();
        b.iter(|| {
            k.hits
                .iter()
                .filter(|key| own.get_mut(key).is_some())
                .count()
        });
    });
    g.bench_function(BenchmarkId::new("insert_all_with_capacity", n), |b| {
        b.iter(|| map_of(n));
    });
    g.bench_function(BenchmarkId::new("insert_all_growing", n), |b| {
        b.iter(|| {
            let mut grown = Map::new();
            for key in &k.hits {
                grown.insert(key.as_str(), Value::Null);
            }
            grown
        });
    });
    g.bench_function(BenchmarkId::new("remove_all", n), |b| {
        b.iter_batched(
            || k.map.clone(),
            |mut own| {
                k.hits
                    .iter()
                    .filter(|key| own.remove(key).is_some())
                    .count()
            },
            criterion::BatchSize::SmallInput,
        );
    });
    g.bench_function(BenchmarkId::new("retain_half", n), |b| {
        b.iter_batched(
            || k.map.clone(),
            |mut own| own.retain(|_, v| v.as_i64().is_some_and(|i| i % 2 == 0)),
            criterion::BatchSize::SmallInput,
        );
    });
    g.finish();
}

fn values(c: &mut Criterion, all: &[Sample]) {
    let sample = all.iter().find(|s| s.name == "config_medium").unwrap();
    let mut g = group(c, "value");
    g.bench_function("get_and_accessors", |b| {
        b.iter(|| {
            let v = black_box(&sample.value);
            let name = v.get("name").and_then(Value::as_str).map(str::len);
            (
                name,
                v.kind(),
                v.is_null(),
                v.as_bool(),
                v.as_i64(),
                v.as_f64(),
                v.as_list().is_none(),
                v.as_map().map(Map::len),
            )
        });
    });
    g.bench_function("clone_medium", |b| {
        b.iter(|| black_box(&sample.value).clone());
    });
    let other = sample.value.clone();
    g.bench_function("eq_medium", |b| {
        b.iter(|| sample.value == black_box(&other).clone());
    });
    g.finish();
    paths(c, &sample.value);
}

/// `Path`, and `Value::at` / `at_mut` along it.
fn paths(c: &mut Criterion, config: &Value) {
    let path = Path::root().key("users").index(0).key("role");
    let tree = Value::Map({
        let mut wrapper = Map::new();
        wrapper.insert(
            "users",
            Value::List(vec![config.get("users").cloned().unwrap_or_default()]),
        );
        wrapper
    });
    let mut g = group(c, "value/path");
    g.bench_function("at_miss_and_hit", |b| {
        b.iter(|| {
            (
                tree.at(black_box(&path)).is_some(),
                tree.at(&Path::root().key("none")).is_some(),
            )
        });
    });
    g.bench_function("at_mut_segments", |b| {
        let mut own = tree.clone();
        let segs = [Seg::Key("users".into()), Seg::Index(0)];
        b.iter(|| own.at_mut(black_box(&segs)).is_some());
    });
    g.bench_function("build_push_pop", |b| {
        b.iter(|| {
            let mut built = Path::root();
            for i in 0..8 {
                built.push(Seg::Key("a".into()));
                built.push(Seg::Index(i));
            }
            let longer = built.key("z").index(3).then(&built);
            built.pop();
            (longer.segments().len(), built.is_root())
        });
    });
    g.finish();
}

fn positions(c: &mut Criterion, all: &[Sample]) {
    let big = all.iter().find(|s| s.name == "records_large").unwrap();
    let doc = text::parse(&big.pretty).unwrap();
    let mut pos = group(c, "text/position");
    pos.throughput(Throughput::Bytes(big.pretty.len() as u64));
    for (label, at) in [
        ("start", 10),
        ("middle", big.pretty.len() / 2),
        ("end", big.pretty.len() - 1),
    ] {
        pos.bench_function(format!("LineCol_of/{label}"), |b| {
            b.iter(|| LineCol::of(black_box(&big.pretty), at));
        });
    }
    let path = Path::root().key("docs").index(1500).key("title");
    pos.bench_function("Doc_spans_at_and_locate", |b| {
        b.iter(|| {
            (
                doc.spans_at(black_box(&path)).span,
                doc.locate(&big.pretty, &path),
            )
        });
    });
    pos.bench_function("Span_new_to", |b| {
        b.iter(|| text::Span::new(black_box(3), 9).to(text::Span::new(5, 20)));
    });
    pos.finish();
}

fn borrowed(c: &mut Criterion, all: &[Sample]) {
    let config = all.iter().find(|s| s.name == "config_large").unwrap();
    let arena = Bump::new();
    let root = binary::decode_ref(&config.bytes, &arena).unwrap();
    let mut g = group(c, "value_ref");
    g.throughput(Throughput::Bytes(config.bytes.len() as u64));
    g.bench_function("to_value", |b| b.iter(|| black_box(root).to_value()));
    g.bench_function("map_get_iter_keys_len", |b| {
        let users = root
            .get("users")
            .and_then(substrate_data::ValueRef::as_map)
            .unwrap();
        b.iter(|| {
            (
                users.get("nobody").is_none(),
                users.iter().count(),
                users.keys().count(),
                users.len(),
                users.is_empty(),
            )
        });
    });
    g.bench_function("accessors", |b| {
        b.iter(|| {
            (
                root.kind(),
                root.is_null(),
                root.as_bool(),
                root.as_i64(),
                root.as_f64(),
                root.as_str(),
                root.as_list().is_none(),
                root.get("name").and_then(substrate_data::ValueRef::as_str),
            )
        });
    });
    g.bench_function("to_ref", |b| {
        let r = Value::Ref(Ref::parse("file", "a").unwrap());
        let scratch = Bump::new();
        b.iter(|| r.borrow(&scratch).to_ref());
    });
    g.finish();
}

fn bench(c: &mut Criterion) {
    let all = vec![
        Sample::new(Topic::Config, Size::Medium),
        Sample::new(Topic::Records, Size::Large),
        Sample::new(Topic::Config, Size::Large),
    ];
    maps(c);
    values(c, &all);
    positions(c, &all);
    borrowed(c, &all);
}

criterion_group! {
    name = benches;
    config = quick();
    targets = bench
}
criterion_main!(benches);
