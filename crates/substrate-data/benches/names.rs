//! Names and small things: references and kind names, typed strings
//! (`classify`, `spelling`, `flatten`), atoms and diagnostics. One group
//! per area; throughput is elements (calls) or the bytes of the string.

#![expect(clippy::unwrap_used, reason = "a benchmark: the inputs are valid")]

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use substrate_data::typed::{self, classify};
use substrate_data::{
    Atom, Code, Diagnostic, KindName, LineCol, Path, Problem, Ref, RefKey, Severity, Value, atom,
    text,
};

#[expect(dead_code, reason = "the corpus is shared; this bench uses part of it")]
mod corpus;

use corpus::each::{group, quick};
use corpus::sample::{Sample, Size, Topic};

fn names(c: &mut Criterion) {
    let mut g = group(c, "names");
    g.bench_function("Ref_parse", |b| {
        b.iter(|| Ref::parse(black_box("file"), black_box("images/logo.png")).unwrap());
    });
    g.bench_function("Ref_new_and_display", |b| {
        let r = Ref::new(
            KindName::new("file").unwrap(),
            RefKey::new("images/logo.png").unwrap(),
        );
        b.iter(|| black_box(&r).to_string());
    });
    g.bench_function("KindName_new_valid", |b| {
        b.iter(|| KindName::new(black_box("user_2")).unwrap());
    });
    g.bench_function("KindName_is_valid_declared_atom", |b| {
        b.iter(|| {
            let k = KindName::declared("doc");
            (
                KindName::is_valid(black_box("doc")),
                k.as_str().len(),
                k.atom().number(),
            )
        });
    });
    g.bench_function("RefKey_new_and_is_valid", |b| {
        b.iter(|| {
            (
                RefKey::new(black_box("images/a/b.png")).unwrap(),
                RefKey::is_valid(black_box("a b")),
            )
        });
    });
    g.finish();
}

fn typed_strings(c: &mut Criterion, all: &[Sample]) {
    let blob = all.iter().find(|s| s.name == "blobs_large").unwrap();
    let spelled = typed::spelling(blob.value.get("bytes").unwrap()).unwrap();
    let tree = all.iter().find(|s| s.name == "refs_large").unwrap();
    let mut g = group(c, "typed");
    for (name, s) in [
        ("plain", "just text"),
        ("escaped_at", "@@everyone"),
        ("ref", "@file:images/logo.png"),
        ("bad_ref", "@File:x"),
    ] {
        g.bench_function(format!("classify/{name}"), |b| {
            b.iter(|| classify(black_box(s)).is_ok());
        });
    }
    g.throughput(Throughput::Bytes(spelled.len() as u64));
    g.bench_function("classify/bytes_375k", |b| {
        b.iter(|| classify(black_box(&spelled)).unwrap());
    });
    g.bench_function("spelling/bytes_375k", |b| {
        b.iter(|| typed::spelling(black_box(blob.value.get("bytes").unwrap())).unwrap());
    });
    g.bench_function("spelling/ref", |b| {
        let r = Value::Ref(Ref::parse("file", "a.png").unwrap());
        b.iter(|| typed::spelling(black_box(&r)));
    });
    g.throughput(Throughput::Bytes(tree.text.len() as u64));
    g.bench_function("flatten/refs_large", |b| {
        b.iter(|| typed::flatten(black_box(&tree.value)));
    });
    g.bench_function("spelled_into_value", |b| {
        b.iter(|| {
            (
                classify(black_box("@@plain")).unwrap().into_value(),
                classify("@file:a/b").unwrap().into_value(),
            )
        });
    });
    g.bench_function("escape_at", |b| {
        b.iter(|| (typed::escape_at(black_box("@x")), typed::escape_at("plain")));
    });
    g.finish();
}

fn atoms(c: &mut Criterion) {
    let words: Vec<String> = (0..10_000).map(|i| format!("atom_word_{i}")).collect();
    let known: Vec<Atom> = words.iter().map(|w| Atom::new(w)).collect();
    let mut g = group(c, "atom");
    g.throughput(Throughput::Elements(words.len() as u64));
    g.bench_function("new_known_10k", |b| {
        b.iter(|| words.iter().map(|w| Atom::new(w).number()).sum::<u32>());
    });
    let mut fresh = 0u64;
    g.bench_function("new_unseen_one", |b| {
        b.iter(|| {
            fresh += 1;
            Atom::new(&format!("fresh_{fresh}")).number()
        });
    });
    g.bench_function("as_str_number_from_number_10k", |b| {
        b.iter(|| {
            known
                .iter()
                .map(|a| Atom::from_number(a.number()).unwrap().as_str().len())
                .sum::<usize>()
        });
    });
    g.bench_function("local_intern_name_10k", |b| {
        b.iter(|| {
            words
                .iter()
                .filter_map(|w| atom::local::name(atom::local::intern(w)))
                .count()
        });
    });
    g.bench_function("display_and_eq_10k", |b| {
        b.iter(|| {
            known
                .iter()
                .map(|a| a.to_string().len() + usize::from(**a == *"x"))
                .sum::<usize>()
        });
    });
    g.finish();
}

fn diagnostics(c: &mut Criterion) {
    let mut g = group(c, "diagnostic");
    g.bench_function("error", |b| {
        b.iter(|| Diagnostic::error(Problem::Expected, black_box("expected a number")));
    });
    g.bench_function("warning_full_chain", |b| {
        b.iter(|| {
            Diagnostic::warning(Problem::UnknownField, "no such field")
                .in_file("projects/alpha/doc.sdt")
                .at_line(12)
                .at(LineCol { line: 12, col: 4 })
                .at_path(Path::root().key("docs").index(3).key("title"))
                .by(Atom::new("linter"))
                .or_code(Problem::Invalid)
                .map(|m| m.to_uppercase())
        });
    });
    let d = Diagnostic::of(Severity::Note, Problem::Syntax, "message").at_line(3);
    g.bench_function("display_and_is_error", |b| {
        b.iter(|| (black_box(&d).to_string(), d.is_error()));
    });
    g.bench_function("syntax_from_error", |b| {
        let e = text::parse_value("{ a: }").unwrap_err();
        b.iter(|| Diagnostic::syntax(black_box(e.clone())));
    });
    g.bench_function("code_tool", |b| {
        b.iter(|| {
            (
                Code::tool(Atom::new("E0308")),
                Code::tool(Atom::new("SD0001")),
            )
        });
    });
    g.bench_function("problem_code_of_code", |b| {
        b.iter(|| {
            Problem::ALL
                .iter()
                .filter_map(|p| Problem::of_code(p.code()))
                .count()
        });
    });
    g.bench_function("parse_error_end_to_end", |b| {
        b.iter(|| text::parse_value(black_box("{ a: [1, 2, }")).unwrap_err());
    });
    g.finish();
}

fn bench(c: &mut Criterion) {
    let all = vec![
        Sample::new(Topic::Blobs, Size::Large),
        Sample::new(Topic::Refs, Size::Large),
    ];
    names(c);
    typed_strings(c, &all);
    atoms(c);
    diagnostics(c);
}

criterion_group! {
    name = benches;
    config = quick();
    targets = bench
}
criterion_main!(benches);
