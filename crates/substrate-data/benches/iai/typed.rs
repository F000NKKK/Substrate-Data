//! Typed documents through every form of this crate: written to a value
//! tree, read from it, binary, frame and text, both ways.

#![expect(
    unused_qualifications,
    reason = "the harness macros qualify the paths of what they expand"
)]
#![expect(clippy::unwrap_used, reason = "a benchmark: the corpus always reads")]

use std::hint::black_box;

use iai_callgrind::{library_benchmark, library_benchmark_group};
use substrate_data::{Schema, Value, frame, from_text, read_bytes, read_value, to_bytes, to_text};

use crate::corpus::typed::{Docs, Numbers, Typed, Users, docs, numbers, users};

/// A document and each form of it, kept for the process.
pub(crate) struct Fixture<T> {
    value: T,
    tree: Value,
    bytes: Vec<u8>,
    frame: Vec<u8>,
    text: String,
}

fn fixture<T: Typed>(value: T) -> &'static Fixture<T> {
    Box::leak(Box::new(Fixture {
        tree: value.write(),
        bytes: to_bytes(&value),
        frame: frame::to_frame(&value),
        text: to_text(&value),
        value,
    }))
}

#[library_benchmark]
#[bench::medium(fixture(users(40)))]
fn users_write(f: &'static Fixture<Users>) -> Value {
    black_box(f.value.write())
}

#[library_benchmark]
#[bench::medium(fixture(users(40)))]
fn users_read_value(f: &'static Fixture<Users>) -> Users {
    black_box(read_value::<Users>(&f.tree).unwrap())
}

#[library_benchmark]
#[bench::medium(fixture(users(40)))]
fn users_to_bytes(f: &'static Fixture<Users>) -> Vec<u8> {
    black_box(to_bytes(&f.value))
}

#[library_benchmark]
#[bench::medium(fixture(users(40)))]
fn users_read_bytes(f: &'static Fixture<Users>) -> Users {
    black_box(read_bytes::<Users>(&f.bytes).unwrap())
}

#[library_benchmark]
#[bench::medium(fixture(users(40)))]
fn users_to_frame(f: &'static Fixture<Users>) -> Vec<u8> {
    black_box(frame::to_frame(&f.value))
}

#[library_benchmark]
#[bench::medium(fixture(users(40)))]
fn users_read_frame(f: &'static Fixture<Users>) -> Users {
    black_box(frame::read_frame::<Users>(&f.frame).unwrap())
}

#[library_benchmark]
#[bench::medium(fixture(users(40)))]
fn users_to_text(f: &'static Fixture<Users>) -> String {
    black_box(to_text(&f.value))
}

#[library_benchmark]
#[bench::medium(fixture(users(40)))]
fn users_from_text(f: &'static Fixture<Users>) -> Users {
    black_box(from_text::<Users>(&f.text).unwrap().value)
}

#[library_benchmark]
#[bench::medium(fixture(docs(30)))]
#[bench::large(fixture(docs(2900)))]
fn docs_write(f: &'static Fixture<Docs>) -> Value {
    black_box(f.value.write())
}

#[library_benchmark]
#[bench::medium(fixture(docs(30)))]
#[bench::large(fixture(docs(2900)))]
fn docs_read_value(f: &'static Fixture<Docs>) -> Docs {
    black_box(read_value::<Docs>(&f.tree).unwrap())
}

#[library_benchmark]
#[bench::medium(fixture(docs(30)))]
#[bench::large(fixture(docs(2900)))]
fn docs_to_bytes(f: &'static Fixture<Docs>) -> Vec<u8> {
    black_box(to_bytes(&f.value))
}

#[library_benchmark]
#[bench::medium(fixture(docs(30)))]
#[bench::large(fixture(docs(2900)))]
fn docs_read_bytes(f: &'static Fixture<Docs>) -> Docs {
    black_box(read_bytes::<Docs>(&f.bytes).unwrap())
}

#[library_benchmark]
#[bench::medium(fixture(docs(30)))]
#[bench::large(fixture(docs(2900)))]
fn docs_to_frame(f: &'static Fixture<Docs>) -> Vec<u8> {
    black_box(frame::to_frame(&f.value))
}

#[library_benchmark]
#[bench::medium(fixture(docs(30)))]
#[bench::large(fixture(docs(2900)))]
fn docs_read_frame(f: &'static Fixture<Docs>) -> Docs {
    black_box(frame::read_frame::<Docs>(&f.frame).unwrap())
}

#[library_benchmark]
#[bench::medium(fixture(docs(30)))]
#[bench::large(fixture(docs(2900)))]
fn docs_to_text(f: &'static Fixture<Docs>) -> String {
    black_box(to_text(&f.value))
}

#[library_benchmark]
#[bench::medium(fixture(docs(30)))]
#[bench::large(fixture(docs(2900)))]
fn docs_from_text(f: &'static Fixture<Docs>) -> Docs {
    black_box(from_text::<Docs>(&f.text).unwrap().value)
}

#[library_benchmark]
#[bench::medium(fixture(numbers(100)))]
#[bench::large(fixture(numbers(10000)))]
fn numbers_write(f: &'static Fixture<Numbers>) -> Value {
    black_box(f.value.write())
}

#[library_benchmark]
#[bench::medium(fixture(numbers(100)))]
#[bench::large(fixture(numbers(10000)))]
fn numbers_read_value(f: &'static Fixture<Numbers>) -> Numbers {
    black_box(read_value::<Numbers>(&f.tree).unwrap())
}

#[library_benchmark]
#[bench::medium(fixture(numbers(100)))]
#[bench::large(fixture(numbers(10000)))]
fn numbers_to_bytes(f: &'static Fixture<Numbers>) -> Vec<u8> {
    black_box(to_bytes(&f.value))
}

#[library_benchmark]
#[bench::medium(fixture(numbers(100)))]
#[bench::large(fixture(numbers(10000)))]
fn numbers_read_bytes(f: &'static Fixture<Numbers>) -> Numbers {
    black_box(read_bytes::<Numbers>(&f.bytes).unwrap())
}

#[library_benchmark]
#[bench::medium(fixture(numbers(100)))]
#[bench::large(fixture(numbers(10000)))]
fn numbers_to_frame(f: &'static Fixture<Numbers>) -> Vec<u8> {
    black_box(frame::to_frame(&f.value))
}

#[library_benchmark]
#[bench::medium(fixture(numbers(100)))]
#[bench::large(fixture(numbers(10000)))]
fn numbers_read_frame(f: &'static Fixture<Numbers>) -> Numbers {
    black_box(frame::read_frame::<Numbers>(&f.frame).unwrap())
}

#[library_benchmark]
#[bench::medium(fixture(numbers(100)))]
#[bench::large(fixture(numbers(10000)))]
fn numbers_to_text(f: &'static Fixture<Numbers>) -> String {
    black_box(to_text(&f.value))
}

#[library_benchmark]
#[bench::medium(fixture(numbers(100)))]
#[bench::large(fixture(numbers(10000)))]
fn numbers_from_text(f: &'static Fixture<Numbers>) -> Numbers {
    black_box(from_text::<Numbers>(&f.text).unwrap().value)
}

library_benchmark_group!(
    name = typed_forms;
    benchmarks =
        users_write,
        users_read_value,
        users_to_bytes,
        users_read_bytes,
        users_to_frame,
        users_read_frame,
        users_to_text,
        users_from_text,
        docs_write,
        docs_read_value,
        docs_to_bytes,
        docs_read_bytes,
        docs_to_frame,
        docs_read_frame,
        docs_to_text,
        docs_from_text,
        numbers_write,
        numbers_read_value,
        numbers_to_bytes,
        numbers_read_bytes,
        numbers_to_frame,
        numbers_read_frame,
        numbers_to_text,
        numbers_from_text
);
