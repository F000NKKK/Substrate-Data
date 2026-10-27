//! The `Reader` and what is built on it: its field helpers, problems,
//! allowances and reference steps; named types; reference ids; shape
//! descriptions; the parsed-document entry points; the `Dictionary` API.

#![expect(clippy::unwrap_used, reason = "a benchmark: the corpus always reads")]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
#[cfg(feature = "toml")]
use substrate_data::schema::from_toml;
use substrate_data::schema::{
    Dictionary, Expected, NamedShape, NamedType, Shape, ShapeDoc, field_name, field_names,
    flattens, from_doc, is_metadata_key, read_both, read_id, read_ref, ref_shape,
};
use substrate_data::with::Plain;
use substrate_data::{
    Bump, KindName, Path, Problem, Reader, Ref, RefKey, Resolve, Seg, Unresolved, Value, binary,
    text,
};

#[expect(dead_code, reason = "the corpus is shared; this bench uses part of it")]
mod corpus;

use corpus::each::{group, quick};
use corpus::shapes::{Doc, Entry, User, Zoo, dictionary, zoo};

/// A resolver that keeps every id as written.
struct Accept;

impl Resolve for Accept {
    fn reference(&mut self, r: &Ref, _path: &Path) -> Result<RefKey, Unresolved> {
        Ok(r.id.clone())
    }
}

/// A named type: a percentage stored as an integer.
#[derive(Debug, PartialEq)]
struct Percent(u8);

impl NamedType for Percent {
    const NAME: &'static str = "percent";
    type Stored = u8;

    fn to_stored(&self) -> u8 {
        self.0
    }

    fn from_stored(stored: u8) -> Option<Self> {
        (stored <= 100).then_some(Percent(stored))
    }
}

substrate_data::named_schema!(Percent);

/// A map value with the fields the field helpers read.
fn object() -> Value {
    let mut m = substrate_data::Map::new();
    m.insert("count", Value::Int(7));
    m.insert("name", Value::Str("meridian".into()));
    m.insert("extra", Value::Bool(true));
    Value::Map(m)
}

/// The helpers that read one field of an object.
fn fields(c: &mut Criterion) {
    let value = object();
    let arena = Bump::new();
    let v = value.borrow(&arena);
    let mut g = group(c, "reader/fields");
    g.bench_function("field", |b| {
        b.iter(|| {
            let mut cx = Reader::new();
            let m = cx.map(black_box(&v)).unwrap();
            cx.field::<i64>(m, "count").unwrap()
        });
    });
    g.bench_function("field_or_missing", |b| {
        b.iter(|| {
            let mut cx = Reader::new();
            let m = cx.map(black_box(&v)).unwrap();
            cx.field_or(m, "absent", || 3_i64).unwrap()
        });
    });
    g.bench_function("field_with", |b| {
        b.iter(|| {
            let mut cx = Reader::new();
            let m = cx.map(black_box(&v)).unwrap();
            cx.field_with::<Plain, String>(m, "name").unwrap()
        });
    });
    g.bench_function("field_or_with_missing", |b| {
        b.iter(|| {
            let mut cx = Reader::new();
            let m = cx.map(black_box(&v)).unwrap();
            cx.field_or_with::<Plain, String>(m, "absent", String::new)
                .unwrap()
        });
    });
    g.bench_function("map_and_unknown_keys", |b| {
        b.iter(|| {
            let mut cx = Reader::new();
            let m = cx.map(black_box(&v)).unwrap();
            cx.unknown_keys(m, &["count", "name"]);
            cx.problems.len()
        });
    });
    g.finish();
}

/// Problems, paths and the scopes a reader keeps.
fn state(c: &mut Criterion) {
    let value = object();
    let arena = Bump::new();
    let v = value.borrow(&arena);
    let mut g = group(c, "reader/state");
    g.bench_function("new", |b| b.iter(Reader::new));
    g.bench_function("at_path_problem", |b| {
        b.iter(|| {
            let mut cx = Reader::new();
            cx.at(Seg::Key("a".into()), |cx| {
                cx.at(Seg::Index(2), |cx| cx.problem("bad"));
            });
            cx.problems.len()
        });
    });
    g.bench_function("problem_of_expected", |b| {
        b.iter(|| {
            let mut cx = Reader::new();
            cx.problem_of(Problem::Invalid, "wrong");
            cx.expected(Expected::Object, black_box(&v));
            cx.unknown_name("calmm", &["calm", "eager"]);
            (
                cx.problems.len(),
                cx.path(),
                cx.keeps_found(),
                cx.resolves(),
            )
        });
    });
    g.bench_function("allowances", |b| {
        b.iter(|| {
            let mut cx = Reader::new();
            cx.with_allowed("type", |cx| {
                cx.with_allowed_all(&["a", "b"], |cx| cx.own_keys(|cx| cx.problems.len()))
            })
        });
    });
    g.bench_function("in_range", |b| {
        b.iter(|| {
            let mut cx = Reader::new();
            let ok = cx.in_range("n", Some(black_box(5_u8)), 0.0, 9.0);
            let out = cx.in_range("n", Some(black_box(50_u8)), 0.0, 9.0);
            (ok, out)
        });
    });
    g.finish();
}

/// Reference steps: kept, resolved, taken, read by kind.
fn references(c: &mut Criterion) {
    let r = Ref::new(KindName::new("user").unwrap(), RefKey::new("ada").unwrap());
    let value = Value::Ref(r.clone());
    let arena = Bump::new();
    let v = value.borrow(&arena);
    let mut g = group(c, "reader/refs");
    g.bench_function("note_ref", |b| {
        b.iter(|| {
            let mut cx = Reader::new();
            cx.note_ref(black_box(&r).clone());
            cx.refs.len()
        });
    });
    g.bench_function("resolve_ref_none", |b| {
        b.iter(|| Reader::new().resolve_ref(black_box(&r)));
    });
    g.bench_function("take_ref_none", |b| {
        b.iter(|| Reader::new().take_ref(black_box(&r).clone()));
    });
    g.bench_function("resolve_ref_resolver", |b| {
        let mut res = Accept;
        b.iter(|| Reader::with_resolver(&mut res).resolve_ref(black_box(&r)));
    });
    g.bench_function("read_id_fast", |b| {
        b.iter(|| read_id::<User>(black_box(&v), &mut Reader::new()).unwrap());
    });
    g.bench_function("read_id_resolved", |b| {
        let mut res = Accept;
        b.iter(|| read_id::<User>(black_box(&v), &mut Reader::with_resolver(&mut res)).unwrap());
    });
    g.bench_function("read_ref_kinds", |b| {
        b.iter(|| read_ref(black_box(&v), &mut Reader::new(), &["file", "user"]).unwrap());
    });
    g.bench_function("expand_ref_none", |b| {
        b.iter(|| Reader::new().expand_ref(black_box(&r)));
    });
    g.bench_function("expand_ref_resolver", |b| {
        let mut res = Accept;
        b.iter(|| Reader::with_resolver(&mut res).expand_ref(black_box(&r)));
    });
    g.bench_function("expanded_nested", |b| {
        b.iter(|| Reader::new().expanded(|cx| cx.expanded(|_| Some(1_u8))));
    });
    g.bench_function("ref_shape", |b| b.iter(ref_shape::<(User, Doc)>));
    g.finish();
}

/// Types read together and named types.
fn composites(c: &mut Criterion) {
    let value = {
        let mut m = substrate_data::Map::new();
        m.insert("title", Value::Str("t".into()));
        m.insert("weight", Value::Float(0.5));
        m.insert("pct", Value::Int(40));
        Value::Map(m)
    };
    let arena = Bump::new();
    let v = value.borrow(&arena);
    let mut g = group(c, "reader/composite");
    g.bench_function("read_both", |b| {
        b.iter(|| read_both::<Entry, Entry>(black_box(&v), &mut Reader::new(), &["pct"]));
    });
    g.bench_function("field_names", |b| {
        let shape = <Entry as substrate_data::Schema>::shape();
        b.iter(|| field_names(black_box(&shape)));
    });
    g.bench_function("is_metadata_key", |b| {
        b.iter(|| {
            ["title", "$schema", "_note", "comment"]
                .iter()
                .filter(|k| is_metadata_key(black_box(k)))
                .count()
        });
    });
    g.bench_function("field_name", |b| {
        const FIELDS: &[(&str, &str)] = &[("type_", "type"), ("name", "name"), ("r#in", "in")];
        b.iter(|| field_name(FIELDS, black_box("r#in")));
    });
    g.bench_function("flattens", |b| b.iter(flattens::<Entry>));
    let pct = Value::Int(40);
    g.bench_function("read_named", |b| {
        b.iter(|| substrate_data::schema::read_value::<Percent>(black_box(&pct)).unwrap());
    });
    g.bench_function("named_shape_stored", |b| {
        b.iter(|| NamedShape::of::<Percent>().stored());
    });
    g.bench_function("shape_named", |b| b.iter(Shape::named::<Percent>));
    g.finish();
}

/// A shape as data.
fn describing(c: &mut Criterion) {
    let shape = <Zoo as substrate_data::Schema>::shape();
    let mut g = group(c, "reader/describe");
    g.bench_function("describe_zoo", |b| b.iter(|| black_box(&shape).describe()));
    g.bench_function("shape_doc_of", |b| {
        b.iter(|| ShapeDoc::of(black_box(&shape)));
    });
    let doc = ShapeDoc::of(&shape);
    g.bench_function("definition", |b| {
        b.iter(|| black_box(&doc).definition("Shaped"));
    });
    g.finish();
}

/// The document entry points and the dictionary's own API.
fn documents(c: &mut Criterion) {
    let value = zoo(4);
    let source = text::to_string(&substrate_data::Schema::write(&value));
    let parsed = text::parse(&source).unwrap();
    let mut g = group(c, "reader/document");
    g.bench_function("from_doc", |b| {
        b.iter(|| from_doc::<Zoo>(black_box(&source), black_box(&parsed)).unwrap());
    });
    #[cfg(feature = "toml")]
    {
        let toml = corpus::typed::toml_users(20);
        g.bench_function("from_toml", |b| {
            b.iter(|| from_toml::<corpus::typed::Users>(black_box(&toml)).unwrap());
        });
    }
    g.finish();
    dictionary_api(c);
}

/// `Dictionary` declarations, kinds and entries.
fn dictionary_api(c: &mut Criterion) {
    let dict = dictionary();
    let file = KindName::new("file").unwrap();
    let mut g = group(c, "reader/dictionary");
    g.bench_function("declare", |b| {
        b.iter(|| {
            let mut d = Dictionary::new();
            d.declare(black_box(file));
            d.holds(file)
        });
    });
    g.bench_function("kinds", |b| b.iter(|| black_box(&dict).kinds().count()));
    g.bench_function("entries", |b| {
        b.iter(|| black_box(&dict).entries(file).count());
    });
    g.bench_function("binary_roundtrip_ref", |b| {
        let bytes = binary::encode(&Value::Int(1));
        b.iter(|| binary::decode(black_box(&bytes)).unwrap());
    });
    g.finish();
}

fn bench(c: &mut Criterion) {
    fields(c);
    state(c);
    references(c);
    composites(c);
    describing(c);
    documents(c);
}

criterion_group! { name = benches; config = quick(); targets = bench }
criterion_main!(benches);
