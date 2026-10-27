//! Instruction counts under valgrind (`iai-callgrind`): deterministic, so
//! they compare paths and revisions on a machine with other work running.
//! Needs `valgrind` and the runner matching the library:
//!
//! ```sh
//! cargo install iai-callgrind-runner --version 0.16.1
//! cargo bench --bench iai
//! ```
//!
//! Five groups: whole documents (text and binary of every topic of the
//! corpus), typed documents in every form, the same typed documents in
//! the serde formats this crate is compared with, and value trees against
//! `serde_json::Value`, and streams. A benchmark runs once,
//! on a document made before the measurement starts.

#![expect(
    clippy::unwrap_used,
    reason = "the corpus builds documents that are valid"
)]

#[expect(dead_code, reason = "the corpus is shared; this bench uses part of it")]
mod corpus;
#[path = "iai/documents.rs"]
mod documents;
#[path = "compare/dynamic.rs"]
mod dynamic;
#[path = "iai/formats.rs"]
mod formats;
#[path = "iai/serde_trees.rs"]
mod serde_trees;
#[path = "iai/streams.rs"]
mod streams;
#[path = "iai/typed.rs"]
mod typed;

use documents::whole_documents;
use formats::serde_formats;
use serde_trees::value_trees;
use streams::stream_forms;
use typed::typed_forms;

iai_callgrind::main!(
    library_benchmark_groups = whole_documents,
    typed_forms,
    serde_formats,
    value_trees,
    stream_forms
);
