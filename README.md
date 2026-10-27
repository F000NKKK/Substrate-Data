# Substrate Data

One data model, one text syntax, one binary form, packed frames and one
schema system — for a program's files, saves, caches and wire messages
alike. Write a format once as a Rust type; read it from hand-edited text
with the line and path of every problem, edit it in place without losing
comments, compile it to compact binary, send it field by field over a
socket, and describe it to an editor that builds property grids from it.

🇺🇸 **English** | 🇷🇺 [Русский](README.ru.md)

```rust
substrate_data::kind!(File = "file", User = "user");

/// A file attached to something.
#[derive(substrate_data::Schema, Debug, PartialEq)]
struct Attachment {
    /// What is attached.
    file: substrate_data::Id<File>,
    #[data(default)]
    owner: Option<substrate_data::Id<User>>,
    #[data(range = "0..=16", default)]
    priority: u8,
}

let r: Attachment = substrate_data::from_text(r#"{
    // The logo for the front page.
    file: "@file:images/logo.png",
    owner: "@user:alice",
}"#).unwrap().value;

let bytes = substrate_data::to_bytes(&r);            // compact, self-describing
let frame = substrate_data::frame::to_frame(&r);     // packed, schema-known
let shape = Attachment::shape().describe();          // for editors and docs
```

## Crates

| Crate | What it is |
|---|---|
| [`substrate-data`](crates/substrate-data) | the model, text/binary/frame forms, schemas, diagnostics |
| [`substrate-data-derive`](crates/substrate-data-derive) | `#[derive(Schema)]` |

Features of `substrate-data`: `json` (JSON ⇄ the model), `toml` (TOML read
into the model with spans).

## The format

[docs/format.md](docs/format.md) is the specification: the model, typed
values (`@kind:id`), the text syntax (JSON + comments, trailing commas,
unquoted keys), the binary layout, frames and their evolution rule,
streams, schemas and their `#[data(...)]` options, and diagnostic codes.

## Performance

On a generated task-tracker document (4000 tasks, 0.99 MB of text,
436 KB binary), release build:

| | substrate-data | serde_json |
|---|---:|---:|
| parse text | 16.1 ms | 19.2 ms |
| parse text with spans | 27.6 ms | — |
| decode binary | 7.1 ms | — |
| write text | 5.1 ms | 2.3 ms |
| encode binary | 2.9 ms | — |

`cargo bench` runs these (`benches/data.rs`) and the typed-message
round trips (`benches/messages.rs`).

## Origin

Extracted from `yog-data` in [Yog Mod Loader](https://github.com/F000NKKK/Yog-Mod-Loader)
with everything Yog-specific removed, so programs such as
[Meridian](https://github.com/F000NKKK/Meridian-Engine), tools and IDEs
built on [Substrate Platform](https://github.com/F000NKKK/Substrate-Platform)
can share it.

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
