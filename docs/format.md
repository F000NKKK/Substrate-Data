# Substrate Data — the format

One data model with four forms, and one schema system over all of them.
Every form carries the same values, so a document saved as text by an
editor, compiled to binary by a build and sent field by field to a running
service is one value throughout.

| Form | For | Self-describing | Keeps comments |
|---|---|---|---|
| [Text](#text) | files people and editors write | yes | yes (patched in place) |
| [Binary](#binary) | cooked content, saves, caches | yes | no |
| [Frame](#frames) | messages both sides know the schema of | no (needs the schema) | no |
| [Stream](#streams) | big lists inside a binary value | yes | no |

The crate is `substrate-data`; `#[derive(Schema)]` comes from
`substrate-data-derive`, re-exported as `substrate_data::Schema`.

---

## Model

A value ([`Value`]) is one of:

| Type | Text | Notes |
|---|---|---|
| null | `null` | |
| bool | `true`, `false` | |
| int | `42`, `-7` | `i64` |
| float | `0.5`, `1e9` | `f64`; a schema may say `f32` |
| string | `"Ruby"` | as spelled — see [Typed values](#typed-values) |
| list | `[1, 2]` | |
| map | `{ a: 1 }` | keys unique, **in file order** |
| ref | `"@file:images/logo.png"` | a reference: `kind` + opaque `id` |
| bytes | `"@bytes:AQID"` | base64 in text, raw in binary |

A reference's **kind** is lower case ASCII (`[a-z][a-z0-9_]*`): `asset`,
`user`, `doc`, `file`, `project`, … — any word a format declares with
`kind!`. Its **id** is anything but empty, whitespace or control
characters; what it names and whether it exists is the format's business
(a [`Resolve`] checks references while reading, and every reference met is
reported with its path).

### Typed values

The text form has no syntax of its own for refs and bytes: they are strings
that start with `@`, so every file stays JSON and every JSON tool still
reads it.

| Written | Value |
|---|---|
| `"@kind:id"` | ref |
| `"@bytes:<base64>"` | bytes |
| `"@@text"` | the plain string `@text` |
| `"@anything else"` | an error (a typo in a reference is never silent text) |

A `Str` in the model holds a string *as spelled*: the parser and the binary
decoder keep `"@file:x"` and `"@@x"` as they are, and the field's schema
decides what it is (`String` reads `"@@x"` as `@x`; `Id<File>` reads
`"@file:x"` as `x`). Writing a `String` doubles a leading `@`, so every
value round-trips through every form.

---

## Text

JSON plus what editors already understand from JSON5:

- `//` and `/* */` comments;
- trailing commas;
- unquoted keys (`{ name: "x" }`).

```jsonc
{
  // The release task.
  name: "Release 1.0",
  project: "@project:apps/notes",
  parts: [
    { type: "estimate", range: [1, 2, 4], parent: "@doc:guides/intro" },
    { type: "attachment", file: "@file:images/logo.png", owner: "@user:alice" },
  ],
}
```

Source-only keys are allowed in every object and ignored by schemas:
`comment`, `$…` (a note on an entry: `$position`), `_…`.

`text::parse` keeps the span of every value and key, so every problem a
schema finds points at its line and column, and its path:

```text
8:33: parts[1].file: `@user:alice` is a reference to user — this takes `@file:`
```

**Editing in place.** `patch::patch(text, &new_value)` rewrites only what
changed and keeps everything else — comments, key order, number spelling,
layout. This is what an inspector uses to write a field back to a file a
person also edits by hand. `patch::format` writes new parts the way
editors write them (short lists and small objects on one line; a whole
float as an integer, any other in the shortest spelling that reads back
as exactly the same `f64` — nothing is rounded).

Writing text, a float is always spelled so it reads back as a float
(`1.0`, `1e16`), never as an integer.

TOML (feature `toml`) and JSON (feature `json`) read into the same model.

---

## Binary

```text
"SDAT" version:u8 (= 1)
value
```

A value is a tag byte and its payload. Strings are interned on the way: a
varint `v`; when odd, `v >> 1` bytes of UTF-8 follow and become the next
table entry; when even, it is table entry `v >> 1`. Repeated keys, kinds
and ids cost one varint after their first use, and the writer never
seeks back.

| Tag | Value | Payload |
|---:|---|---|
| 0 | null | — |
| 1, 2 | false, true | — |
| 3 | int | zigzag LEB128 varint |
| 4 | float | f64 LE |
| 5 | string | string |
| 6 | list | varint count, values |
| 7 | map | varint count, (key string, value) pairs |
| 8 | ref | kind string, id string |
| 9 | bytes | varint length, bytes |
| 10 | list of ints, packed | varint count, zigzag varints |
| 11 | stream | u32 LE byte length, then varint count and values with their own string table |
| 12 | float, 32-bit | f32 LE |
| 13 | list of floats, packed | varint count, f64 LE each |
| 14 | list of 32-bit floats, packed | varint count, f32 LE each |

Nothing is aligned. Decoding checks every count and length against the
bytes left and nesting against `MAX_DEPTH` (256): malformed input is an
error, never a panic or a huge allocation.

Two decoders: `binary::decode` builds an owned `Value`;
`binary::decode_ref` borrows every string from the input and puts lists
and maps in a `bumpalo` arena. Derived schemas encode and decode
themselves straight (`to_bytes` / `read_bytes`) with no value tree in
between, falling back to the checked slow path only to explain an error.

---

## Frames

A frame is a value packed for a message whose schema both sides know:
nothing says what a value is or what it's called.

```text
frame  = body, string table, table length (a varint written backwards)
record = varint length, its fields in declaration order
```

| Value | Packed |
|---|---|
| bool | a byte |
| int | zigzag varint |
| `f64`, `f32` | 8, 4 bytes LE |
| string | varint index into the table |
| bytes | varint length, bytes |
| `Option` | a byte (0/1), then the value |
| list (and set, `OneOrList`, `Stream`, `Path`) | varint count, varint byte length, items |
| list of ints | count, bit width (a byte), values bit-packed in lanes |
| list of `f64` | count, `f32`-exact flag (a byte), raw values (4 bytes each when the flag is set, else 8) |
| list of `f32` | count, raw values, 4 bytes each |
| tuple | as a list, each item its binary form as bytes |
| unit enum | varint variant index |
| [`Atom`] | its number in this process's table |
| anything else | its binary form, as bytes |

**Evolution rule:** fields are only ever appended. A record carries its
length, so a reader that knows fewer fields skips the rest and one that
knows more takes their defaults. Variants are only ever appended too.

`frame::to_frame` / `read_frame` for owned values; `#[data(view)]` on a
struct generates a zero-copy view (`frame::View`) over a frame's bytes.
`frame::read_shaped(bytes, &shape)` reads a frame by its [`Shape`] alone,
for a tool that has no code for the message (an editor's message
inspector).

`io::write_frame` / `read_frame` put frames on a byte stream (a socket, a
pipe): each message a little-endian `u32` length, then the frame.

---

## Streams

`Stream<T>` is a list kept in the binary form: items written as they come
and read back one at a time when asked, with its own string table, so it
can be cut out of a document whole. For the big ones: a table's rows, a
picture's pixels, a recording, a log.

---

## Schemas

A format is a Rust type implementing [`Schema`], usually derived:

```rust
substrate_data::kind!(File = "file", User = "user", Doc = "doc");

/// How long it takes.
#[derive(substrate_data::Schema)]
struct Estimate {
    /// Low, likely and high, in days.
    range: [f32; 3],
    #[data(default)]
    parent: Option<substrate_data::Id<Doc>>,
}

/// A part of a task.
#[derive(substrate_data::Schema)]
#[data(tag = "type")]
enum Part {
    Estimate(Estimate),
    Attachment { file: substrate_data::Id<File>, owner: Option<substrate_data::Id<User>> },
    Archived,
}
```

One schema gives:

- **read** — from text (with line, column and path of every problem, an
  unknown field's nearest known name suggested), from a value, from binary;
- **write** — to a value, text, binary (`encode`) and frames (`pack`);
- **references** — every `@kind:id` met, with its path, checked by a
  [`Resolve`] when one is given;
- **shape** — `T::shape()` describes the type (fields, docs from `///`,
  required or not, numeric ranges, enum variants), and `Shape::describe()`
  turns it into a value any tool can read: an editor builds property grids,
  pickers (a `ref` field of kind `file` gets the file picker), completion
  and hover docs from it; docs generate their field tables from it.

`#[data(...)]` options:

| Option | On | Meaning |
|---|---|---|
| `rename = "x"` | field, variant | its key / name |
| `rename_all = "camelCase"` | struct, enum | every field / variant |
| `default`, `default = "fn"` | field | optional, with `Default` or `fn()` |
| `skip` | field | never read or written (runtime state) |
| `range = "lo..=hi"` | field | numbers checked and described |
| `with = "Codec"` | field | read/written through a codec (`with::RefTo<K>`, `with::Opt<…>`, `with::List<…>`, `with::Raw`, …) |
| `tag = "type"` | enum | tagged by this key (the default for enums with data) |
| `view` | struct | also generate a frame view |
| `other` | variant | a tagged enum's catch-all: wraps a type that reads the whole object (tag included) for any tag no other variant names |
| `crate = "path"` | struct, enum | where `substrate_data` is, for crates that re-export it |

A tagged enum's variant that wraps a struct lays the struct's fields next
to the tag; one that wraps anything else keeps it under `value`.

### Domain types

`Shape::Named { name, of }` lets a format describe a type of its own
(`"color"`, `"date"`, `"money"`, `"duration"`) stored as plain data
(`of`): a tool that knows the name shows its own editor (a colour picker,
a calendar), one that doesn't edits `of`. A program implements `Schema`
for its own types in its own adapter crate and returns a named shape; this crate knows nothing
of them.

A named type is stored as `of` in every form: its `write`, `encode` and
`pack` produce what `of`'s would (so `frame::read_shaped` reads it as
`of`). A list of a named type packs item by item (the default
`pack_list`), not as a packed list of numbers.

---

## Diagnostics

Every problem is a `Diagnostic`: a code, a severity, a message, and where
(file, path, line and column). Codes from this crate:

| Code | Meaning |
|---|---|
| SD0000 | any other problem with a value |
| SD0001 | text that doesn't parse |
| SD0002 | a value of another type than its field's |
| SD0003 | a bare id where a reference goes |
| SD0004 | a reference of another kind than its field takes |
| SD0005 | unknown field |
| SD0006 | missing field |
| SD0007 | number out of range |
| SD0008 | a string written as a typed value that isn't one |
| SD0009 | binary data that won't decode |

Tools add their own codes; a `Diagnostic` is itself a `Schema`, so it
crosses a protocol like any other message.

---

## Origin

Extracted from `yog-data` (Yog Mod Loader), with everything specific to
Yog removed: content namespaces, tags, mod resources, translation keys,
Yog text, image sources and colours as built-in typed values. Those become a domain layer over this crate. The binary tags and
magic were renumbered (`SDAT` v1); nothing reads the old `YOGD` bytes.

[`Value`]: ../crates/substrate-data/src/model/value.rs
[`Schema`]: ../crates/substrate-data/src/schema/mod.rs
[`Shape`]: ../crates/substrate-data/src/schema/shape.rs
[`Resolve`]: ../crates/substrate-data/src/schema/reader.rs
[`Atom`]: ../crates/substrate-data/src/model/atom.rs
