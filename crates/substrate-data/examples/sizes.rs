//! Encoded sizes: the same corpus in this crate's forms and in the formats
//! it is compared with (`benches/compare.rs` measures their times), and
//! where the bytes of the binary form go. Prints Markdown tables:
//!
//! ```sh
//! cargo run --release --example sizes
//! ```

#![expect(
    clippy::print_stdout,
    clippy::unwrap_used,
    clippy::cast_precision_loss,
    reason = "an example that prints tables of sizes"
)]

#[expect(
    dead_code,
    reason = "the corpus is shared; this example uses part of it"
)]
#[path = "../benches/corpus/mod.rs"]
mod corpus;
#[path = "../benches/compare/dynamic.rs"]
mod dynamic;
#[path = "sizes/walk.rs"]
mod walk;

use corpus::formats::Fmt;
use corpus::sample::{Size, samples};
use corpus::typed::{Typed, Visit, each};
use walk::{Spent, spent};

/// Prints one row per typed document: its size in every format.
struct Table;

impl Visit for Table {
    fn visit<T: Typed>(&mut self, name: &str, value: &T) {
        let cells: Vec<String> = Fmt::ALL
            .iter()
            .map(|f| {
                let bytes = f.encode(value);
                corpus::formats::check(*f, value, &bytes, name);
                bytes.len().to_string()
            })
            .collect();
        println!("| {name} | {} |", cells.join(" | "));
    }
}

/// Where the bytes of each typed document's `to_bytes` go.
struct Breakdown;

impl Visit for Breakdown {
    fn visit<T: Typed>(&mut self, name: &str, value: &T) {
        row(
            &format!("{name} (to_bytes)"),
            &substrate_data::to_bytes(value),
        );
    }
}

fn typed_table() {
    println!("### Typed documents, bytes\n");
    let heads: Vec<String> = Fmt::ALL.iter().map(ToString::to_string).collect();
    println!("| document | {} |", heads.join(" | "));
    println!("|---|{}", "---:|".repeat(heads.len()));
    each(&mut Table, &Size::ALL);
}

fn dynamic_table() {
    println!("\n### Value trees, bytes\n");
    println!(
        "| document | sd_text | sd_text pretty | sd_binary | json | cbor | msgpack | sd_binary / json |"
    );
    println!("|---|---:|---:|---:|---:|---:|---:|---:|");
    for s in samples() {
        let tree = dynamic::tree(&s.value);
        let json = serde_json::to_vec(&tree).unwrap().len();
        let mut cbor = Vec::new();
        ciborium::into_writer(&tree, &mut cbor).unwrap();
        let msgpack = rmp_serde::to_vec_named(&tree).unwrap().len();
        println!(
            "| {} | {} | {} | {} | {json} | {} | {msgpack} | {:.0} % |",
            s.name,
            s.text.len(),
            s.pretty.len(),
            s.bytes.len(),
            cbor.len(),
            s.bytes.len() as f64 * 100.0 / json as f64
        );
    }
}

fn share(part: usize, total: usize) -> String {
    format!("{:.0}", part as f64 * 100.0 / total.max(1) as f64)
}

fn row(name: &str, bytes: &[u8]) {
    let s: Spent = spent(bytes).expect("every document walks");
    assert_eq!(s.total(), bytes.len(), "{name}: every byte is counted once");
    let t = bytes.len();
    println!(
        "| {name} | {t} | {} | {} | {} | {} | {} | {} | {} | {} |",
        share(s.tags, t),
        share(s.counts, t),
        share(s.key_literal, t),
        share(s.key_ref, t),
        share(s.str_literal, t),
        share(s.str_ref, t),
        share(s.ints, t),
        share(s.floats + s.blobs, t),
    );
}

fn breakdown_table() {
    println!("\n### Where the bytes of the binary form go (% of the document)\n");
    println!(
        "| document | bytes | tags | counts | key literals | key refs | string literals | string refs | ints | floats + blobs |"
    );
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for s in samples().iter().filter(|s| s.size != Size::Small) {
        row(&format!("{} (Value)", s.name), &s.bytes);
    }
    each(&mut Breakdown, &[Size::Medium, Size::Large]);
}

fn main() {
    typed_table();
    dynamic_table();
    breakdown_table();
}
