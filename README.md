<div align="center">

<a id="top"></a>

# 🧱 Substrate Data

### One data model, text, binary, frames and schemas: for a program's files, saves and messages

[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)
[![Rust 1.92+](https://img.shields.io/badge/Rust-1.92%2B-orange.svg)](Cargo.toml)
[![crates.io](https://img.shields.io/crates/v/substrate-data.svg)](https://crates.io/crates/substrate-data)

🇺🇸 **English** | 🇷🇺 [Русский](README.ru.md)

[Overview](#-overview) • [Getting Started](#-getting-started) • [Architecture](#-architecture) • [Development](#-development) • [License](#-license)

</div>

---

## 📖 Overview

Write a format once as a Rust type. Read it from hand-edited text with the line and path of every problem, edit it in place without losing comments, compile it to compact binary, send it field by field over a socket, and describe it to an editor that builds property grids from it. The library knows no product: domain types (users, docs, files, projects) live in the code of whoever uses it.

| Part | Where | What it gives you |
|---|---|---|
| Model | [`substrate-data`](crates/substrate-data) | JSON's values plus typed references `@kind:id` and bytes |
| Text | [`substrate-data`](crates/substrate-data) | JSON with comments, trailing commas and unquoted keys; the position of every value; in-place edits |
| Binary | [`substrate-data`](crates/substrate-data) | the compact, self-describing form |
| Frames | [`substrate-data`](crates/substrate-data) | the packed form for messages whose schema both sides know |
| Schema | [`substrate-data`](crates/substrate-data) | a format as a type: read with the path of every problem, written, described for editors |
| Derive | [`substrate-data-derive`](crates/substrate-data-derive) | `#[derive(Schema)]` |

The format specification: [docs/format.md](docs/format.md). What may change in a type between versions, proven by tests: [docs/evolution.md](docs/evolution.md). Language-neutral test vectors for ports are in [conformance/](conformance/README.md). How it is benchmarked, and the numbers: [docs/performance.md](docs/performance.md).

## 🚀 Getting Started

```toml
# Cargo.toml
[dependencies]
substrate-data = "0.2"
```

Features of `substrate-data`: `json` (JSON ⇄ the model), `toml` (TOML read into the model with spans), `tokio` (frames over async streams, `io::asynchronous`).

```rust
use substrate_data::{Id, Schema};

substrate_data::kind!(File = "file", User = "user": u64);

/// A file attached to something.
#[derive(Schema, Debug, PartialEq)]
struct Attachment {
    /// What is attached.
    file: Id<File>,
    #[data(default)]
    owner: Option<Id<User>>,
    #[data(range = "0..=16", default)]
    priority: u8,
}

let attachment: Attachment = substrate_data::from_text(r#"{
    // The logo for the front page.
    file: "@file:images/logo.png",
    owner: "@user:42",
}"#)
.unwrap()
.value;
assert_eq!(attachment.priority, 0);

let bytes = substrate_data::to_bytes(&attachment); // compact, self-describing
let frame = substrate_data::frame::to_frame(&attachment); // packed, schema-known
let shape = Attachment::shape().describe(); // a typed ShapeDoc, for editors and docs
```

## 🏛️ Architecture

Both crates are the shared kernel (context `data`): they depend on no other crate of the project.

```text
substrate-data-derive   #[derive(Schema)]: parses a type, generates the impl
        ▲
substrate-data          model → text / binary / frames → schemas and diagnostics
```

`substrate-data` is built from the model outwards: text, binary and frames read and write the same model, and a schema on top gives typed reads with diagnostics. The details (the text grammar, binary tags, frame layout, shape description keys, diagnostic codes) are in [docs/format.md](docs/format.md).

## 🛠️ Development

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo bench --no-run
python3 <craftbook>/tools/craftcheck check . --baseline .craftcheck-baseline
python3 <craftbook>/tools/craftcheck deps . --baseline .craftcheck-deps-baseline
```

Engineering standard: [Craftbook](https://github.com/F000NKKK/craftbook). Changes: [CHANGELOG.md](CHANGELOG.md).

## 📄 License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.

---

<div align="center">

**[⬆ Back to Top](#top)**

</div>
