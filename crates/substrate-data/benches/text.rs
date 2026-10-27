//! The text form and what reads and writes whole files: parsing (values,
//! with spans, with comments), writing (compact, pretty, a string), the
//! patch (an edited document, the rest kept), `source` (a file as bytes),
//! and, with their features, JSON and TOML. One group per path, every
//! sample of the corpus in each; throughput is the bytes of the text.

#![expect(clippy::unwrap_used, reason = "a benchmark: the corpus always parses")]

use criterion::{Criterion, criterion_group, criterion_main};
use substrate_data::{patch, source, text};

#[expect(dead_code, reason = "the corpus is shared; this bench uses part of it")]
mod corpus;

use corpus::each::{per_sample, quick};
use corpus::sample::{Sample, commented, edited, samples};

fn parse(c: &mut Criterion, all: &[Sample]) {
    per_sample(
        c,
        "text/parse_value",
        all,
        |s| s.text.len(),
        |s| text::parse_value(&s.text).unwrap(),
    );
    per_sample(
        c,
        "text/parse_pretty",
        all,
        |s| s.pretty.len(),
        |s| text::parse_value(&s.pretty).unwrap(),
    );
    per_sample(
        c,
        "text/parse_with_spans",
        all,
        |s| s.text.len(),
        |s| text::parse(&s.text).unwrap(),
    );
    let notes: Vec<String> = all.iter().map(|s| commented(&s.pretty)).collect();
    let by_name = |s: &Sample| notes[all.iter().position(|o| o.name == s.name).unwrap()].len();
    per_sample(c, "text/with_comments", all, by_name, |s| {
        let at = all.iter().position(|o| o.name == s.name).unwrap();
        text::with_comments(&notes[at]).unwrap()
    });
}

fn write(c: &mut Criterion, all: &[Sample]) {
    per_sample(
        c,
        "text/to_string",
        all,
        |s| s.text.len(),
        |s| text::to_string(&s.value),
    );
    per_sample(
        c,
        "text/to_string_pretty",
        all,
        |s| s.pretty.len(),
        |s| text::to_string_pretty(&s.value),
    );
    let strings: Vec<&Sample> = all
        .iter()
        .filter(|s| s.name.starts_with("strings"))
        .collect();
    let mut g = c.benchmark_group("text/write_string");
    for s in strings {
        let source = s
            .value
            .get("items")
            .map_or(String::new(), |v| format!("{v:?}"));
        g.throughput(criterion::Throughput::Bytes(source.len() as u64));
        g.bench_function(&s.name, |b| {
            b.iter(|| {
                let mut out = String::with_capacity(source.len() + 8);
                text::write_string(std::hint::black_box(&source), &mut out);
                out
            });
        });
    }
    g.finish();
}

fn edit(c: &mut Criterion, all: &[Sample]) {
    let next: Vec<_> = all.iter().map(|s| edited(&s.value)).collect();
    let at = |s: &Sample| all.iter().position(|o| o.name == s.name).unwrap();
    per_sample(
        c,
        "patch/patch",
        all,
        |s| s.pretty.len(),
        |s| patch::patch(&s.pretty, &next[at(s)]),
    );
    per_sample(
        c,
        "patch/format",
        all,
        |s| s.pretty.len(),
        |s| patch::format(&s.value, 0),
    );
    per_sample(
        c,
        "patch/to_json",
        all,
        |s| s.text.len(),
        |s| patch::to_json(&s.value),
    );
    per_sample(
        c,
        "patch/same",
        all,
        |s| s.text.len(),
        |s| patch::same(&s.value, &next[at(s)]),
    );
}

fn sources(c: &mut Criterion, all: &[Sample]) {
    per_sample(
        c,
        "source/text_from_text",
        all,
        |s| s.text.len(),
        |s| source::text(s.text.as_bytes()).unwrap(),
    );
    per_sample(
        c,
        "source/text_from_binary",
        all,
        |s| s.bytes.len(),
        |s| source::text(&s.bytes).unwrap(),
    );
    per_sample(
        c,
        "source/binary_from_text",
        all,
        |s| s.pretty.len(),
        |s| source::binary(&s.pretty).unwrap(),
    );
    per_sample(
        c,
        "source/is_binary",
        all,
        |s| s.bytes.len(),
        |s| source::is_binary(&s.bytes),
    );
}

#[cfg(feature = "json")]
fn json(c: &mut Criterion, all: &[Sample]) {
    use substrate_data::json;
    let trees: Vec<_> = all.iter().map(|s| json::to_json(&s.value)).collect();
    let at = |s: &Sample| all.iter().position(|o| o.name == s.name).unwrap();
    per_sample(
        c,
        "json/from_json_text",
        all,
        |s| s.text.len(),
        |s| json::from_json_text(&s.text),
    );
    per_sample(
        c,
        "json/from_json",
        all,
        |s| s.text.len(),
        |s| json::from_json(&trees[at(s)]),
    );
    per_sample(
        c,
        "json/to_json",
        all,
        |s| s.text.len(),
        |s| json::to_json(&s.value),
    );
}

#[cfg(feature = "toml")]
fn toml(c: &mut Criterion) {
    let mut g = c.benchmark_group("toml/parse");
    for (name, n) in [("small", 1), ("medium", 40), ("large", 4000)] {
        let text = corpus::typed::toml_users(n);
        g.throughput(criterion::Throughput::Bytes(text.len() as u64));
        g.bench_function(name, |b| {
            b.iter(|| substrate_data::toml::parse(&text).unwrap());
        });
    }
    g.finish();
}

fn bench(c: &mut Criterion) {
    let all = samples();
    parse(c, &all);
    write(c, &all);
    edit(c, &all);
    sources(c, &all);
    #[cfg(feature = "json")]
    json(c, &all);
    #[cfg(feature = "toml")]
    toml(c);
}

criterion_group! {
    name = benches;
    config = quick();
    targets = bench
}
criterion_main!(benches);
