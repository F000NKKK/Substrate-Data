# substrate-data

One data model, text syntax, binary form, packed frames and schema system — for any program's files, saves and wire messages.

**Layer and context:** shared kernel · data

- **For:** the model with typed references, the text form (JSON with comments, spans, in-place patches), the binary form, packed frames and streams, schemas with `#[derive(Schema)]`, and diagnostics with codes and paths.
- **Not for:** domain types or any product's vocabulary; those belong in the consumer's adapter crate (`impl Schema` + `Shape::Named`). The format itself is specified in [docs/format.md](../../docs/format.md).

## Usage

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

## Features

| Feature | Enables |
|---|---|
| `json` | JSON read into and written from the model |
| `toml` | TOML read into the model, with spans |
| `simd-utf8` | SIMD UTF-8 validation (`simdutf8`, runtime-dispatched) where bytes become text: source files, the binary form, frames. Errors and results are identical; it pays on long strings and blobs |

The text parser's scans (string bodies, comments, line starts) always use
`memchr`, which picks SSE2/AVX2/NEON at run time.

## Depends on / used by

- Depends on: [`substrate-data-derive`](../substrate-data-derive) (re-exported as `Schema`)
- Used by: programs and adapter crates that define their own formats

## License

MIT OR Apache-2.0, see [LICENSE-MIT](../../LICENSE-MIT) and [LICENSE-APACHE](../../LICENSE-APACHE).
