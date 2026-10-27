//! Whole documents: the text form and the binary form of every topic of the
//! corpus, in the medium (about 5 KB) and large (about 500 KB) sizes.

#![expect(
    unused_qualifications,
    reason = "the harness macros qualify the paths of what they expand"
)]
#![expect(clippy::unwrap_used, reason = "a benchmark: the corpus always parses")]

use std::hint::black_box;

use iai_callgrind::{library_benchmark, library_benchmark_group};
use substrate_data::{Bump, Value, binary, patch, source, text};

use crate::corpus::sample::{Sample, Size, Topic};

/// The document of `topic` and `size`, kept for the process (so dropping it is not measured).
pub(crate) fn sample(topic: Topic, size: Size) -> &'static Sample {
    Box::leak(Box::new(Sample::new(topic, size)))
}

/// A document and the same with one more entry, for the patch.
pub(crate) fn edit_of(topic: Topic, size: Size) -> (&'static Sample, &'static Value) {
    let s = sample(topic, size);
    (
        s,
        Box::leak(Box::new(crate::corpus::sample::edited(&s.value))),
    )
}

/// A pretty text with comments, for the comment-keeping parse.
pub(crate) fn commented_of(topic: Topic, size: Size) -> &'static str {
    let s = sample(topic, size);
    Box::leak(crate::corpus::sample::commented(&s.pretty).into_boxed_str())
}

#[library_benchmark]
#[bench::config_medium(args = (Topic::Config, Size::Medium), setup = sample)]
#[bench::config_large(args = (Topic::Config, Size::Large), setup = sample)]
#[bench::records_medium(args = (Topic::Records, Size::Medium), setup = sample)]
#[bench::records_large(args = (Topic::Records, Size::Large), setup = sample)]
#[bench::numbers_medium(args = (Topic::Numbers, Size::Medium), setup = sample)]
#[bench::numbers_large(args = (Topic::Numbers, Size::Large), setup = sample)]
#[bench::strings_medium(args = (Topic::Strings, Size::Medium), setup = sample)]
#[bench::strings_large(args = (Topic::Strings, Size::Large), setup = sample)]
#[bench::deep_medium(args = (Topic::Deep, Size::Medium), setup = sample)]
#[bench::deep_large(args = (Topic::Deep, Size::Large), setup = sample)]
#[bench::refs_medium(args = (Topic::Refs, Size::Medium), setup = sample)]
#[bench::refs_large(args = (Topic::Refs, Size::Large), setup = sample)]
#[bench::blobs_medium(args = (Topic::Blobs, Size::Medium), setup = sample)]
#[bench::blobs_large(args = (Topic::Blobs, Size::Large), setup = sample)]
fn text_parse_value(s: &'static Sample) -> Value {
    black_box(text::parse_value(&s.text).unwrap())
}

#[library_benchmark]
#[bench::config_medium(args = (Topic::Config, Size::Medium), setup = sample)]
#[bench::config_large(args = (Topic::Config, Size::Large), setup = sample)]
#[bench::records_medium(args = (Topic::Records, Size::Medium), setup = sample)]
#[bench::records_large(args = (Topic::Records, Size::Large), setup = sample)]
#[bench::numbers_medium(args = (Topic::Numbers, Size::Medium), setup = sample)]
#[bench::numbers_large(args = (Topic::Numbers, Size::Large), setup = sample)]
#[bench::strings_medium(args = (Topic::Strings, Size::Medium), setup = sample)]
#[bench::strings_large(args = (Topic::Strings, Size::Large), setup = sample)]
#[bench::deep_medium(args = (Topic::Deep, Size::Medium), setup = sample)]
#[bench::deep_large(args = (Topic::Deep, Size::Large), setup = sample)]
#[bench::refs_medium(args = (Topic::Refs, Size::Medium), setup = sample)]
#[bench::refs_large(args = (Topic::Refs, Size::Large), setup = sample)]
#[bench::blobs_medium(args = (Topic::Blobs, Size::Medium), setup = sample)]
#[bench::blobs_large(args = (Topic::Blobs, Size::Large), setup = sample)]
fn text_to_string(s: &'static Sample) -> String {
    black_box(text::to_string(&s.value))
}

#[library_benchmark]
#[bench::config_medium(args = (Topic::Config, Size::Medium), setup = sample)]
#[bench::config_large(args = (Topic::Config, Size::Large), setup = sample)]
#[bench::records_medium(args = (Topic::Records, Size::Medium), setup = sample)]
#[bench::records_large(args = (Topic::Records, Size::Large), setup = sample)]
#[bench::numbers_medium(args = (Topic::Numbers, Size::Medium), setup = sample)]
#[bench::numbers_large(args = (Topic::Numbers, Size::Large), setup = sample)]
#[bench::strings_medium(args = (Topic::Strings, Size::Medium), setup = sample)]
#[bench::strings_large(args = (Topic::Strings, Size::Large), setup = sample)]
#[bench::deep_medium(args = (Topic::Deep, Size::Medium), setup = sample)]
#[bench::deep_large(args = (Topic::Deep, Size::Large), setup = sample)]
#[bench::refs_medium(args = (Topic::Refs, Size::Medium), setup = sample)]
#[bench::refs_large(args = (Topic::Refs, Size::Large), setup = sample)]
#[bench::blobs_medium(args = (Topic::Blobs, Size::Medium), setup = sample)]
#[bench::blobs_large(args = (Topic::Blobs, Size::Large), setup = sample)]
fn binary_encode(s: &'static Sample) -> Vec<u8> {
    black_box(binary::encode(&s.value))
}

#[library_benchmark]
#[bench::config_medium(args = (Topic::Config, Size::Medium), setup = sample)]
#[bench::config_large(args = (Topic::Config, Size::Large), setup = sample)]
#[bench::records_medium(args = (Topic::Records, Size::Medium), setup = sample)]
#[bench::records_large(args = (Topic::Records, Size::Large), setup = sample)]
#[bench::numbers_medium(args = (Topic::Numbers, Size::Medium), setup = sample)]
#[bench::numbers_large(args = (Topic::Numbers, Size::Large), setup = sample)]
#[bench::strings_medium(args = (Topic::Strings, Size::Medium), setup = sample)]
#[bench::strings_large(args = (Topic::Strings, Size::Large), setup = sample)]
#[bench::deep_medium(args = (Topic::Deep, Size::Medium), setup = sample)]
#[bench::deep_large(args = (Topic::Deep, Size::Large), setup = sample)]
#[bench::refs_medium(args = (Topic::Refs, Size::Medium), setup = sample)]
#[bench::refs_large(args = (Topic::Refs, Size::Large), setup = sample)]
#[bench::blobs_medium(args = (Topic::Blobs, Size::Medium), setup = sample)]
#[bench::blobs_large(args = (Topic::Blobs, Size::Large), setup = sample)]
fn binary_decode(s: &'static Sample) -> Value {
    black_box(binary::decode(&s.bytes).unwrap())
}

#[library_benchmark]
#[bench::config_medium(args = (Topic::Config, Size::Medium), setup = sample)]
#[bench::records_medium(args = (Topic::Records, Size::Medium), setup = sample)]
#[bench::numbers_medium(args = (Topic::Numbers, Size::Medium), setup = sample)]
#[bench::strings_medium(args = (Topic::Strings, Size::Medium), setup = sample)]
#[bench::deep_medium(args = (Topic::Deep, Size::Medium), setup = sample)]
#[bench::refs_medium(args = (Topic::Refs, Size::Medium), setup = sample)]
#[bench::blobs_medium(args = (Topic::Blobs, Size::Medium), setup = sample)]
fn text_parse_with_spans(s: &'static Sample) -> text::Doc {
    black_box(text::parse(&s.text).unwrap())
}

#[library_benchmark]
#[bench::config_medium(args = (Topic::Config, Size::Medium), setup = sample)]
#[bench::records_medium(args = (Topic::Records, Size::Medium), setup = sample)]
#[bench::numbers_medium(args = (Topic::Numbers, Size::Medium), setup = sample)]
#[bench::strings_medium(args = (Topic::Strings, Size::Medium), setup = sample)]
#[bench::deep_medium(args = (Topic::Deep, Size::Medium), setup = sample)]
#[bench::refs_medium(args = (Topic::Refs, Size::Medium), setup = sample)]
#[bench::blobs_medium(args = (Topic::Blobs, Size::Medium), setup = sample)]
fn text_to_string_pretty(s: &'static Sample) -> String {
    black_box(text::to_string_pretty(&s.value))
}

#[library_benchmark]
#[bench::config_medium(args = (Topic::Config, Size::Medium), setup = sample)]
#[bench::records_medium(args = (Topic::Records, Size::Medium), setup = sample)]
#[bench::numbers_medium(args = (Topic::Numbers, Size::Medium), setup = sample)]
#[bench::strings_medium(args = (Topic::Strings, Size::Medium), setup = sample)]
#[bench::deep_medium(args = (Topic::Deep, Size::Medium), setup = sample)]
#[bench::refs_medium(args = (Topic::Refs, Size::Medium), setup = sample)]
#[bench::blobs_medium(args = (Topic::Blobs, Size::Medium), setup = sample)]
fn text_parse_pretty(s: &'static Sample) -> Value {
    black_box(text::parse_value(&s.pretty).unwrap())
}

#[library_benchmark]
#[bench::config_medium(args = (Topic::Config, Size::Medium), setup = sample)]
#[bench::records_medium(args = (Topic::Records, Size::Medium), setup = sample)]
#[bench::numbers_medium(args = (Topic::Numbers, Size::Medium), setup = sample)]
#[bench::strings_medium(args = (Topic::Strings, Size::Medium), setup = sample)]
#[bench::deep_medium(args = (Topic::Deep, Size::Medium), setup = sample)]
#[bench::refs_medium(args = (Topic::Refs, Size::Medium), setup = sample)]
#[bench::blobs_medium(args = (Topic::Blobs, Size::Medium), setup = sample)]
fn binary_decode_ref_to_value(s: &'static Sample) -> Value {
    black_box(
        binary::decode_ref(&s.bytes, &Bump::new())
            .unwrap()
            .to_value(),
    )
}

#[library_benchmark]
#[bench::config_medium(args = (Topic::Config, Size::Medium), setup = sample)]
#[bench::records_medium(args = (Topic::Records, Size::Medium), setup = sample)]
#[bench::numbers_medium(args = (Topic::Numbers, Size::Medium), setup = sample)]
#[bench::strings_medium(args = (Topic::Strings, Size::Medium), setup = sample)]
#[bench::deep_medium(args = (Topic::Deep, Size::Medium), setup = sample)]
#[bench::refs_medium(args = (Topic::Refs, Size::Medium), setup = sample)]
#[bench::blobs_medium(args = (Topic::Blobs, Size::Medium), setup = sample)]
fn source_text_from_binary(s: &'static Sample) -> String {
    black_box(source::text(&s.bytes).unwrap())
}

#[library_benchmark]
#[bench::config_medium(args = (Topic::Config, Size::Medium), setup = sample)]
#[bench::records_medium(args = (Topic::Records, Size::Medium), setup = sample)]
#[bench::numbers_medium(args = (Topic::Numbers, Size::Medium), setup = sample)]
#[bench::strings_medium(args = (Topic::Strings, Size::Medium), setup = sample)]
#[bench::deep_medium(args = (Topic::Deep, Size::Medium), setup = sample)]
#[bench::refs_medium(args = (Topic::Refs, Size::Medium), setup = sample)]
#[bench::blobs_medium(args = (Topic::Blobs, Size::Medium), setup = sample)]
fn source_binary_from_text(s: &'static Sample) -> Vec<u8> {
    black_box(source::binary(&s.pretty).unwrap())
}

#[library_benchmark]
#[bench::config_medium(args = (Topic::Config, Size::Medium), setup = sample)]
#[bench::records_medium(args = (Topic::Records, Size::Medium), setup = sample)]
#[bench::numbers_medium(args = (Topic::Numbers, Size::Medium), setup = sample)]
#[bench::strings_medium(args = (Topic::Strings, Size::Medium), setup = sample)]
#[bench::deep_medium(args = (Topic::Deep, Size::Medium), setup = sample)]
#[bench::refs_medium(args = (Topic::Refs, Size::Medium), setup = sample)]
#[bench::blobs_medium(args = (Topic::Blobs, Size::Medium), setup = sample)]
fn patch_format(s: &'static Sample) -> String {
    black_box(patch::format(&s.value, 0))
}

#[library_benchmark]
#[bench::records_medium(args = (Topic::Records, Size::Medium), setup = edit_of)]
#[bench::config_medium(args = (Topic::Config, Size::Medium), setup = edit_of)]
#[bench::refs_medium(args = (Topic::Refs, Size::Medium), setup = edit_of)]
fn patch_edit((s, next): (&'static Sample, &'static Value)) -> String {
    black_box(patch::patch(&s.pretty, next))
}

#[library_benchmark]
#[bench::records_medium(args = (Topic::Records, Size::Medium), setup = commented_of)]
#[bench::config_medium(args = (Topic::Config, Size::Medium), setup = commented_of)]
#[bench::deep_medium(args = (Topic::Deep, Size::Medium), setup = commented_of)]
fn text_with_comments(text: &'static str) -> Value {
    black_box(text::with_comments(text).unwrap())
}

#[library_benchmark]
#[bench::config_large(args = (Topic::Config, Size::Large), setup = sample)]
#[bench::records_large(args = (Topic::Records, Size::Large), setup = sample)]
#[bench::numbers_large(args = (Topic::Numbers, Size::Large), setup = sample)]
#[bench::deep_large(args = (Topic::Deep, Size::Large), setup = sample)]
fn text_parse_ref(s: &'static Sample) -> usize {
    let arena = Bump::new();
    black_box(text::parse_ref(&s.text, &arena).unwrap().kind() as usize)
}

library_benchmark_group!(
    name = whole_documents;
    benchmarks = text_parse_value, text_parse_ref, text_to_string, binary_encode, binary_decode, text_parse_with_spans, text_to_string_pretty, text_parse_pretty, binary_decode_ref_to_value, source_text_from_binary, source_binary_from_text, patch_format, patch_edit, text_with_comments
);
