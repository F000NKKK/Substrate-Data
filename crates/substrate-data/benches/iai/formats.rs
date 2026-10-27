//! The same typed documents in this crate's forms and in the serde formats
//! it is compared with: instruction counts of encoding and decoding.

#![expect(
    unused_qualifications,
    reason = "the harness macros qualify the paths of what they expand"
)]
#![expect(
    clippy::unwrap_used,
    reason = "a benchmark: the corpus always round-trips"
)]

use std::hint::black_box;

use iai_callgrind::{library_benchmark, library_benchmark_group};

use crate::corpus::formats::Fmt;
use crate::corpus::typed::{Docs, Numbers, Typed, Users, docs, numbers, users};

/// A document and its encoding in a format, kept for the process.
pub(crate) struct Encoded<T> {
    format: Fmt,
    value: T,
    bytes: Vec<u8>,
}

fn encoded<T: Typed>(format: Fmt, value: T) -> &'static Encoded<T> {
    let bytes = format.encode(&value);
    Box::leak(Box::new(Encoded {
        format,
        value,
        bytes,
    }))
}

#[library_benchmark]
#[bench::medium_sdbinary(encoded(Fmt::SdBinary, users(40)))]
#[bench::medium_sdframe(encoded(Fmt::SdFrame, users(40)))]
#[bench::medium_sdtext(encoded(Fmt::SdText, users(40)))]
#[bench::medium_json(encoded(Fmt::Json, users(40)))]
#[bench::medium_cbor(encoded(Fmt::Cbor, users(40)))]
#[bench::medium_msgpacknamed(encoded(Fmt::MsgpackNamed, users(40)))]
#[bench::medium_msgpack(encoded(Fmt::Msgpack, users(40)))]
#[bench::medium_postcard(encoded(Fmt::Postcard, users(40)))]
#[bench::medium_bincode(encoded(Fmt::Bincode, users(40)))]
fn users_encode(e: &'static Encoded<Users>) -> Vec<u8> {
    black_box(e.format.encode(&e.value))
}

#[library_benchmark]
#[bench::medium_sdbinary(encoded(Fmt::SdBinary, users(40)))]
#[bench::medium_sdframe(encoded(Fmt::SdFrame, users(40)))]
#[bench::medium_sdtext(encoded(Fmt::SdText, users(40)))]
#[bench::medium_json(encoded(Fmt::Json, users(40)))]
#[bench::medium_cbor(encoded(Fmt::Cbor, users(40)))]
#[bench::medium_msgpacknamed(encoded(Fmt::MsgpackNamed, users(40)))]
#[bench::medium_msgpack(encoded(Fmt::Msgpack, users(40)))]
#[bench::medium_postcard(encoded(Fmt::Postcard, users(40)))]
#[bench::medium_bincode(encoded(Fmt::Bincode, users(40)))]
fn users_decode(e: &'static Encoded<Users>) -> Users {
    black_box(e.format.decode::<Users>(&e.bytes).unwrap())
}

#[library_benchmark]
#[bench::medium_sdbinary(encoded(Fmt::SdBinary, docs(30)))]
#[bench::medium_sdframe(encoded(Fmt::SdFrame, docs(30)))]
#[bench::medium_sdtext(encoded(Fmt::SdText, docs(30)))]
#[bench::medium_json(encoded(Fmt::Json, docs(30)))]
#[bench::medium_cbor(encoded(Fmt::Cbor, docs(30)))]
#[bench::medium_msgpacknamed(encoded(Fmt::MsgpackNamed, docs(30)))]
#[bench::medium_msgpack(encoded(Fmt::Msgpack, docs(30)))]
#[bench::medium_postcard(encoded(Fmt::Postcard, docs(30)))]
#[bench::medium_bincode(encoded(Fmt::Bincode, docs(30)))]
#[bench::large_sdbinary(encoded(Fmt::SdBinary, docs(2900)))]
#[bench::large_sdframe(encoded(Fmt::SdFrame, docs(2900)))]
#[bench::large_sdtext(encoded(Fmt::SdText, docs(2900)))]
#[bench::large_json(encoded(Fmt::Json, docs(2900)))]
#[bench::large_cbor(encoded(Fmt::Cbor, docs(2900)))]
#[bench::large_msgpacknamed(encoded(Fmt::MsgpackNamed, docs(2900)))]
#[bench::large_msgpack(encoded(Fmt::Msgpack, docs(2900)))]
#[bench::large_postcard(encoded(Fmt::Postcard, docs(2900)))]
#[bench::large_bincode(encoded(Fmt::Bincode, docs(2900)))]
fn docs_encode(e: &'static Encoded<Docs>) -> Vec<u8> {
    black_box(e.format.encode(&e.value))
}

#[library_benchmark]
#[bench::medium_sdbinary(encoded(Fmt::SdBinary, docs(30)))]
#[bench::medium_sdframe(encoded(Fmt::SdFrame, docs(30)))]
#[bench::medium_sdtext(encoded(Fmt::SdText, docs(30)))]
#[bench::medium_json(encoded(Fmt::Json, docs(30)))]
#[bench::medium_cbor(encoded(Fmt::Cbor, docs(30)))]
#[bench::medium_msgpacknamed(encoded(Fmt::MsgpackNamed, docs(30)))]
#[bench::medium_msgpack(encoded(Fmt::Msgpack, docs(30)))]
#[bench::medium_postcard(encoded(Fmt::Postcard, docs(30)))]
#[bench::medium_bincode(encoded(Fmt::Bincode, docs(30)))]
#[bench::large_sdbinary(encoded(Fmt::SdBinary, docs(2900)))]
#[bench::large_sdframe(encoded(Fmt::SdFrame, docs(2900)))]
#[bench::large_sdtext(encoded(Fmt::SdText, docs(2900)))]
#[bench::large_json(encoded(Fmt::Json, docs(2900)))]
#[bench::large_cbor(encoded(Fmt::Cbor, docs(2900)))]
#[bench::large_msgpacknamed(encoded(Fmt::MsgpackNamed, docs(2900)))]
#[bench::large_msgpack(encoded(Fmt::Msgpack, docs(2900)))]
#[bench::large_postcard(encoded(Fmt::Postcard, docs(2900)))]
#[bench::large_bincode(encoded(Fmt::Bincode, docs(2900)))]
fn docs_decode(e: &'static Encoded<Docs>) -> Docs {
    black_box(e.format.decode::<Docs>(&e.bytes).unwrap())
}

#[library_benchmark]
#[bench::medium_sdbinary(encoded(Fmt::SdBinary, numbers(100)))]
#[bench::medium_sdframe(encoded(Fmt::SdFrame, numbers(100)))]
#[bench::medium_sdtext(encoded(Fmt::SdText, numbers(100)))]
#[bench::medium_json(encoded(Fmt::Json, numbers(100)))]
#[bench::medium_cbor(encoded(Fmt::Cbor, numbers(100)))]
#[bench::medium_msgpacknamed(encoded(Fmt::MsgpackNamed, numbers(100)))]
#[bench::medium_msgpack(encoded(Fmt::Msgpack, numbers(100)))]
#[bench::medium_postcard(encoded(Fmt::Postcard, numbers(100)))]
#[bench::medium_bincode(encoded(Fmt::Bincode, numbers(100)))]
#[bench::large_sdbinary(encoded(Fmt::SdBinary, numbers(10000)))]
#[bench::large_sdframe(encoded(Fmt::SdFrame, numbers(10000)))]
#[bench::large_sdtext(encoded(Fmt::SdText, numbers(10000)))]
#[bench::large_json(encoded(Fmt::Json, numbers(10000)))]
#[bench::large_cbor(encoded(Fmt::Cbor, numbers(10000)))]
#[bench::large_msgpacknamed(encoded(Fmt::MsgpackNamed, numbers(10000)))]
#[bench::large_msgpack(encoded(Fmt::Msgpack, numbers(10000)))]
#[bench::large_postcard(encoded(Fmt::Postcard, numbers(10000)))]
#[bench::large_bincode(encoded(Fmt::Bincode, numbers(10000)))]
fn numbers_encode(e: &'static Encoded<Numbers>) -> Vec<u8> {
    black_box(e.format.encode(&e.value))
}

#[library_benchmark]
#[bench::medium_sdbinary(encoded(Fmt::SdBinary, numbers(100)))]
#[bench::medium_sdframe(encoded(Fmt::SdFrame, numbers(100)))]
#[bench::medium_sdtext(encoded(Fmt::SdText, numbers(100)))]
#[bench::medium_json(encoded(Fmt::Json, numbers(100)))]
#[bench::medium_cbor(encoded(Fmt::Cbor, numbers(100)))]
#[bench::medium_msgpacknamed(encoded(Fmt::MsgpackNamed, numbers(100)))]
#[bench::medium_msgpack(encoded(Fmt::Msgpack, numbers(100)))]
#[bench::medium_postcard(encoded(Fmt::Postcard, numbers(100)))]
#[bench::medium_bincode(encoded(Fmt::Bincode, numbers(100)))]
#[bench::large_sdbinary(encoded(Fmt::SdBinary, numbers(10000)))]
#[bench::large_sdframe(encoded(Fmt::SdFrame, numbers(10000)))]
#[bench::large_sdtext(encoded(Fmt::SdText, numbers(10000)))]
#[bench::large_json(encoded(Fmt::Json, numbers(10000)))]
#[bench::large_cbor(encoded(Fmt::Cbor, numbers(10000)))]
#[bench::large_msgpacknamed(encoded(Fmt::MsgpackNamed, numbers(10000)))]
#[bench::large_msgpack(encoded(Fmt::Msgpack, numbers(10000)))]
#[bench::large_postcard(encoded(Fmt::Postcard, numbers(10000)))]
#[bench::large_bincode(encoded(Fmt::Bincode, numbers(10000)))]
fn numbers_decode(e: &'static Encoded<Numbers>) -> Numbers {
    black_box(e.format.decode::<Numbers>(&e.bytes).unwrap())
}

library_benchmark_group!(
    name = serde_formats;
    benchmarks = users_encode, users_decode, docs_encode, docs_decode, numbers_encode, numbers_decode
);
