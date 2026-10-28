//! Schemas: every way to read and write a derived type (value, text,
//! binary, frame), the read paths with a resolver, `Expand` and a
//! `Dictionary`, the plain types and field codecs, shape descriptions,
//! the `Reader`, and streams. One group per area; throughput is the bytes
//! of the form measured (or elements, for the in-memory paths).

#![expect(clippy::unwrap_used, reason = "a benchmark: the corpus always reads")]

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use substrate_data::schema::Dictionary;
use substrate_data::{
    Diagnostic, Path, Ref, RefKey, Resolve, Schema, Stream, Unresolved, Value, binary, frame,
    from_bytes_with, from_doc_with, from_text, from_value, from_value_with, read_bytes,
    read_ref_value, read_value, text, to_bytes, to_bytes_into, to_text,
};

#[expect(dead_code, reason = "the corpus is shared; this bench uses part of it")]
mod corpus;

use corpus::each::{group, quick};
use corpus::sample::Size;
use corpus::shapes::{
    Attachments, Cards, Zoo, attachments, cards, dictionary, plains, wrappeds, zoo,
};
use corpus::typed::{Typed, Visit, each};

/// Every typed document, through every way of reading and writing it.
struct Forms<'a>(&'a mut Criterion);

impl Visit for Forms<'_> {
    fn visit<T: Typed>(&mut self, name: &str, value: &T) {
        forms(self.0, name, value);
    }
}

/// `value` through the value and binary forms and back.
fn forms<T: Schema + PartialEq + std::fmt::Debug>(c: &mut Criterion, name: &str, value: &T) {
    let tree = value.write();
    let bytes = to_bytes(value);
    assert_eq!(&read_bytes::<T>(&bytes).unwrap(), value);
    assert_eq!(&from_text::<T>(&to_text(value)).unwrap().value, value);
    let mut g = group(c, "schema/forms");
    g.throughput(Throughput::Bytes(bytes.len() as u64));
    g.bench_function(format!("write/{name}"), |b| {
        b.iter(|| black_box(value).write());
    });
    g.bench_function(format!("read_value/{name}"), |b| {
        b.iter(|| read_value::<T>(black_box(&tree)).unwrap());
    });
    g.bench_function(format!("from_value/{name}"), |b| {
        b.iter(|| from_value::<T>(black_box(&tree)).unwrap().value);
    });
    g.bench_function(format!("to_bytes/{name}"), |b| {
        b.iter(|| to_bytes(black_box(value)));
    });
    let mut out = Vec::new();
    g.bench_function(format!("to_bytes_into_reused/{name}"), |b| {
        b.iter(|| {
            out.clear();
            to_bytes_into(black_box(value), &mut out);
        });
    });
    g.bench_function(format!("encode_via_value/{name}"), |b| {
        b.iter(|| binary::encode(&black_box(value).write()));
    });
    g.bench_function(format!("read_bytes/{name}"), |b| {
        b.iter(|| read_bytes::<T>(black_box(&bytes)).unwrap());
    });
    g.bench_function(format!("read_ref_value/{name}"), |b| {
        b.iter(|| {
            let arena = substrate_data::Bump::new();
            let v = binary::decode_ref(black_box(&bytes), &arena).unwrap();
            read_ref_value::<T>(&v).unwrap()
        });
    });
    g.finish();
    forms_text(c, name, value);
}

/// `value` as text and as a frame, and back.
fn forms_text<T: Schema + PartialEq + std::fmt::Debug>(c: &mut Criterion, name: &str, value: &T) {
    let text = to_text(value);
    let packed = frame::to_frame(value);
    let mut g = group(c, "schema/forms");
    g.throughput(Throughput::Bytes(text.len() as u64));
    g.bench_function(format!("to_text/{name}"), |b| {
        b.iter(|| to_text(black_box(value)));
    });
    g.bench_function(format!("from_text/{name}"), |b| {
        b.iter(|| from_text::<T>(black_box(&text)).unwrap().value);
    });
    g.throughput(Throughput::Bytes(packed.len() as u64));
    g.bench_function(format!("to_frame/{name}"), |b| {
        b.iter(|| frame::to_frame(black_box(value)));
    });
    g.bench_function(format!("read_frame/{name}"), |b| {
        b.iter(|| frame::read_frame::<T>(black_box(&packed)).unwrap());
    });
    g.finish();
}

/// The derive's own shapes: every kind of field, in every form.
fn derived_shapes(c: &mut Criterion) {
    for (label, n) in [("small", 1), ("medium", 50), ("large", 2000)] {
        forms(c, &format!("zoo_{label}"), &zoo(n));
    }
    forms(c, "plains", &plains());
    forms(c, "wrapped_medium", &wrappeds(50));
    forms(c, "attachments_medium", &attachments(50));
}

/// A resolver that accepts every reference as it is.
struct Accept(usize);

impl Resolve for Accept {
    fn reference(&mut self, r: &Ref, _: &Path) -> Result<RefKey, Unresolved> {
        self.0 += 1;
        Ok(r.id.clone())
    }
}

/// Reading references: with no resolver, a resolver that accepts, and a
/// `Dictionary` that checks and expands them.
fn resolving(c: &mut Criterion) {
    let mut dict = dictionary();
    for (label, n) in [("small", 1), ("medium", 55), ("large", 5400)] {
        let docs = attachments(n);
        let tree = docs.write();
        let bytes = to_bytes(&docs);
        let text = text::to_string(&tree);
        let parsed = text::parse(&text).unwrap();
        let mut g = group(c, "schema/resolve");
        g.throughput(Throughput::Elements(n as u64));
        g.bench_function(format!("none/{label}"), |b| {
            b.iter(|| read_value::<Attachments>(black_box(&tree)).unwrap());
        });
        g.bench_function(format!("accept_all/{label}"), |b| {
            b.iter(|| from_value_with::<Attachments>(black_box(&tree), &mut Accept(0)).unwrap());
        });
        g.bench_function(format!("dictionary_value/{label}"), |b| {
            b.iter(|| from_value_with::<Attachments>(black_box(&tree), &mut dict).unwrap());
        });
        g.bench_function(format!("dictionary_bytes/{label}"), |b| {
            b.iter(|| from_bytes_with::<Attachments>(black_box(&bytes), &mut dict).unwrap());
        });
        g.bench_function(format!("dictionary_doc/{label}"), |b| {
            b.iter(|| from_doc_with::<Attachments>(&text, black_box(&parsed), &mut dict).unwrap());
        });
        g.finish();
        expanding(c, label, n, &mut dict);
    }
}

/// `Expand`: references replaced by the values a dictionary holds.
fn expanding(c: &mut Criterion, label: &str, n: usize, dict: &mut Dictionary) {
    let docs = cards(n);
    let tree = docs.write();
    let bytes = to_bytes(&docs);
    let mut g = group(c, "schema/expand");
    g.throughput(Throughput::Elements(n as u64));
    g.bench_function(format!("kept_no_resolver/{label}"), |b| {
        b.iter(|| read_value::<Cards>(black_box(&tree)).unwrap());
    });
    g.bench_function(format!("expanded_value/{label}"), |b| {
        b.iter(|| from_value_with::<Cards>(black_box(&tree), dict).unwrap());
    });
    g.bench_function(format!("expanded_bytes/{label}"), |b| {
        b.iter(|| from_bytes_with::<Cards>(black_box(&bytes), dict).unwrap());
    });
    g.finish();
}

/// A document with a mistake in every record: the reading that reports.
fn failing(c: &mut Criterion) {
    let mut tree = zoo(200).write();
    if let Value::Map(m) = &mut tree
        && let Some(Value::List(items)) = m.get_mut("items")
    {
        for item in items {
            if let Value::Map(record) = item {
                record.insert("count", Value::Str("many".into()));
                record.insert("surprise", Value::Null);
            }
        }
    }
    let text = text::to_string(&tree);
    let mut g = group(c, "schema/diagnostics");
    g.throughput(Throughput::Elements(200));
    g.bench_function("read_value_two_problems_each", |b| {
        b.iter(|| read_value::<Zoo>(black_box(&tree)).unwrap_err().len());
    });
    g.bench_function("from_text_placed", |b| {
        b.iter(|| from_text::<Zoo>(black_box(&text)).unwrap_err().0.len());
    });
    let doc = text::parse(&text).unwrap();
    let one = Diagnostic::error(substrate_data::Problem::Syntax, "a problem");
    g.bench_function("placed_one", |b| {
        b.iter(|| substrate_data::placed(&text, &doc, black_box(one.clone())));
    });
    g.finish();
}

/// Streams: lists kept in the binary form, written one by one, read lazily.
fn streams(c: &mut Criterion) {
    let n = 10_000u32;
    let mut g = group(c, "stream");
    g.throughput(Throughput::Elements(u64::from(n)));
    g.bench_function("writer_push_finish", |b| {
        b.iter(|| {
            let mut w = Stream::<u32>::writer();
            for i in 0..n {
                w.push(&i);
            }
            w.finish()
        });
    });
    let s: Stream<u32> = (0..n).collect();
    g.bench_function("from_iter", |b| b.iter(|| (0..n).collect::<Stream<u32>>()));
    g.bench_function("iter", |b| {
        b.iter(|| {
            black_box(&s)
                .iter()
                .map(|x| u64::from(x.unwrap()))
                .sum::<u64>()
        });
    });
    g.bench_function("to_vec", |b| b.iter(|| black_box(&s).to_vec().unwrap()));
    g.bench_function("len_bytes_len", |b| {
        b.iter(|| black_box(&s).len() + s.bytes_len());
    });
    g.finish();
}

fn bench(c: &mut Criterion) {
    each(&mut Forms(c), &Size::ALL);
    derived_shapes(c);
    resolving(c);
    failing(c);
    streams(c);
}

criterion_group! {
    name = benches;
    config = quick();
    targets = bench
}
criterion_main!(benches);
