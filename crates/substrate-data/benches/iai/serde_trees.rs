//! Value trees against `serde_json::Value` (JSON, CBOR and `MessagePack`
//! all go through it): instruction counts of parsing and writing the same
//! corpus documents.

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
use serde_json::Value as Tree;

use crate::corpus::sample::{Sample, Size, Topic};
use crate::dynamic;

/// A document as a `serde_json` tree and in each of its formats, kept for the process.
pub(crate) struct Trees {
    tree: Tree,
    json: Vec<u8>,
    cbor: Vec<u8>,
    msgpack: Vec<u8>,
}

fn tree(topic: Topic, size: Size) -> &'static Trees {
    let tree = dynamic::tree(&Sample::new(topic, size).value);
    let mut cbor = Vec::new();
    ciborium::into_writer(&tree, &mut cbor).unwrap();
    Box::leak(Box::new(Trees {
        json: serde_json::to_vec(&tree).unwrap(),
        msgpack: rmp_serde::to_vec_named(&tree).unwrap(),
        cbor,
        tree,
    }))
}

#[library_benchmark]
#[bench::config_medium(args = (Topic::Config, Size::Medium), setup = tree)]
#[bench::config_large(args = (Topic::Config, Size::Large), setup = tree)]
#[bench::records_medium(args = (Topic::Records, Size::Medium), setup = tree)]
#[bench::records_large(args = (Topic::Records, Size::Large), setup = tree)]
#[bench::numbers_medium(args = (Topic::Numbers, Size::Medium), setup = tree)]
#[bench::numbers_large(args = (Topic::Numbers, Size::Large), setup = tree)]
#[bench::strings_medium(args = (Topic::Strings, Size::Medium), setup = tree)]
#[bench::strings_large(args = (Topic::Strings, Size::Large), setup = tree)]
#[bench::deep_medium(args = (Topic::Deep, Size::Medium), setup = tree)]
#[bench::deep_large(args = (Topic::Deep, Size::Large), setup = tree)]
#[bench::refs_medium(args = (Topic::Refs, Size::Medium), setup = tree)]
#[bench::refs_large(args = (Topic::Refs, Size::Large), setup = tree)]
#[bench::blobs_medium(args = (Topic::Blobs, Size::Medium), setup = tree)]
#[bench::blobs_large(args = (Topic::Blobs, Size::Large), setup = tree)]
fn json_parse(t: &'static Trees) -> Tree {
    black_box(serde_json::from_slice::<Tree>(&t.json).unwrap())
}

#[library_benchmark]
#[bench::config_medium(args = (Topic::Config, Size::Medium), setup = tree)]
#[bench::config_large(args = (Topic::Config, Size::Large), setup = tree)]
#[bench::records_medium(args = (Topic::Records, Size::Medium), setup = tree)]
#[bench::records_large(args = (Topic::Records, Size::Large), setup = tree)]
#[bench::numbers_medium(args = (Topic::Numbers, Size::Medium), setup = tree)]
#[bench::numbers_large(args = (Topic::Numbers, Size::Large), setup = tree)]
#[bench::strings_medium(args = (Topic::Strings, Size::Medium), setup = tree)]
#[bench::strings_large(args = (Topic::Strings, Size::Large), setup = tree)]
#[bench::deep_medium(args = (Topic::Deep, Size::Medium), setup = tree)]
#[bench::deep_large(args = (Topic::Deep, Size::Large), setup = tree)]
#[bench::refs_medium(args = (Topic::Refs, Size::Medium), setup = tree)]
#[bench::refs_large(args = (Topic::Refs, Size::Large), setup = tree)]
#[bench::blobs_medium(args = (Topic::Blobs, Size::Medium), setup = tree)]
#[bench::blobs_large(args = (Topic::Blobs, Size::Large), setup = tree)]
fn json_write(t: &'static Trees) -> Vec<u8> {
    black_box(serde_json::to_vec(&t.tree).unwrap())
}

#[library_benchmark]
#[bench::config_medium(args = (Topic::Config, Size::Medium), setup = tree)]
#[bench::config_large(args = (Topic::Config, Size::Large), setup = tree)]
#[bench::records_medium(args = (Topic::Records, Size::Medium), setup = tree)]
#[bench::records_large(args = (Topic::Records, Size::Large), setup = tree)]
#[bench::numbers_medium(args = (Topic::Numbers, Size::Medium), setup = tree)]
#[bench::numbers_large(args = (Topic::Numbers, Size::Large), setup = tree)]
fn cbor_decode(t: &'static Trees) -> Tree {
    black_box(ciborium::from_reader::<Tree, _>(&t.cbor[..]).unwrap())
}

#[library_benchmark]
#[bench::config_medium(args = (Topic::Config, Size::Medium), setup = tree)]
#[bench::config_large(args = (Topic::Config, Size::Large), setup = tree)]
#[bench::records_medium(args = (Topic::Records, Size::Medium), setup = tree)]
#[bench::records_large(args = (Topic::Records, Size::Large), setup = tree)]
#[bench::numbers_medium(args = (Topic::Numbers, Size::Medium), setup = tree)]
#[bench::numbers_large(args = (Topic::Numbers, Size::Large), setup = tree)]
fn cbor_encode(t: &'static Trees) -> Vec<u8> {
    {
        let mut out = Vec::new();
        ciborium::into_writer(&t.tree, &mut out).unwrap();
        black_box(out)
    }
}

#[library_benchmark]
#[bench::config_medium(args = (Topic::Config, Size::Medium), setup = tree)]
#[bench::config_large(args = (Topic::Config, Size::Large), setup = tree)]
#[bench::records_medium(args = (Topic::Records, Size::Medium), setup = tree)]
#[bench::records_large(args = (Topic::Records, Size::Large), setup = tree)]
#[bench::numbers_medium(args = (Topic::Numbers, Size::Medium), setup = tree)]
#[bench::numbers_large(args = (Topic::Numbers, Size::Large), setup = tree)]
fn msgpack_decode(t: &'static Trees) -> Tree {
    black_box(rmp_serde::from_slice::<Tree>(&t.msgpack).unwrap())
}

#[library_benchmark]
#[bench::config_medium(args = (Topic::Config, Size::Medium), setup = tree)]
#[bench::config_large(args = (Topic::Config, Size::Large), setup = tree)]
#[bench::records_medium(args = (Topic::Records, Size::Medium), setup = tree)]
#[bench::records_large(args = (Topic::Records, Size::Large), setup = tree)]
#[bench::numbers_medium(args = (Topic::Numbers, Size::Medium), setup = tree)]
#[bench::numbers_large(args = (Topic::Numbers, Size::Large), setup = tree)]
fn msgpack_encode(t: &'static Trees) -> Vec<u8> {
    black_box(rmp_serde::to_vec_named(&t.tree).unwrap())
}

library_benchmark_group!(
    name = value_trees;
    benchmarks = json_parse, json_write, cbor_decode, cbor_encode, msgpack_decode, msgpack_encode
);
