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

Golden test vectors for ports in other languages — exact bytes of every
tag, frame and malformed input — are in [`conformance/`](../conformance/README.md).

---

## Model

A value ([`Value`]) is one of:

| Type | Text | Notes |
|---|---|---|
| null | `null` | |
| bool | `true`, `false` | |
| int | `42`, `-7` | `i64`; a `u64` above `i64::MAX` is carried as its bits (`u64::MAX` is `-1`) |
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
reported with its path). In code both parts are checked newtypes,
`KindName` (interned) and `RefKey`, made once when the reference is read.

A kind declares the **type of its ids**: `kind!(User = "user": u64, Doc =
"doc")` — `String` when none is said, else `u32`, `u64`, `i64` or `Uid`
(128 bits, written as 32 lower-case hex digits), or any type implementing
`RefId`. `Id<User>` holds a `u64`; `"@user:abc"` is an SD0010 problem at
its path. The text and binary forms spell every id as text, so changing a
kind's id type changes no file.

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
editors write them (short lists and small objects on one line; a float in
the shortest spelling that reads back as exactly the same `f64` and as a
float — `1.0`, `-0.0`, `0.1` — nothing is rounded). A number whose value
didn't change keeps the source's spelling (`1` stays `1` for `1.0`); `-0.0`
and `0` count as different.

Writing text, a float is always spelled so it reads back as a float
(`1.0`, `1e16`), never as an integer. The spelling is Rust's `{:?}` for
`f64`: the shortest digits that read back as the same bits, in positional
form (`0.0001`, `123456789012345.0`) while `1e-4 <= |f| < 1e16`, otherwise
in exponent form with no `+` and no padding (`1e16`, `5e-324`, `1e-7`); zero
is `0.0` or `-0.0`. A port must produce the same text (the conformance
vectors check it); a language's own float printer usually differs.

`"@bytes:..."` is canonical standard base64: the `+/` alphabet, `=` padding
to a multiple of four, and no stray bits in the last character. Anything
else is a syntax error, not a lenient read.

TOML (feature `toml`) and JSON (feature `json`) read into the same model.

---

## Binary

```text
"SDAT" version:u8 (= 2)
value
```

**Version 2 is the only version written.** Every writer (the reference's,
a port's) writes version 2 and nothing else: there is no version 1 writer
and no mode that keeps a document's version. Readers read version 1 and
version 2; any other version byte is the error `bad_version`. Reading a
version 1 document and writing the value back upgrades it, and loses
nothing: every version 1 document is a value of the same model, and every
value has a version 2 form (`binary::upgrade(bytes)` is `decode` then
`encode`). A reader that sees a tag the document's version does not have
fails with `unknown_tag`; it never skips or guesses. Binary tags only grow:
a tag's number is never reused, and version 1's tags (0 to 14) mean in
version 2 what they meant.

A value is a tag byte and its payload. Strings are interned on the way: a
varint `v`; when odd, `v >> 1` bytes of UTF-8 follow and become the next
table entry; when even, it is table entry `v >> 1`. Repeated keys, kinds
and ids cost one varint after their first use, and the writer never
seeks back. The odd marker's low bit alone is `binary::NEW_STRING` (`1`),
public so decoders in other languages take it from here.

A varint is LEB128 of a `u64`: at most ten bytes, the tenth `0` or `1`. A
longer one, or one that doesn't fit 64 bits, is malformed.

| Tag | Value | Payload | Version |
|---:|---|---|---|
| 0 | null | — | 1 |
| 1, 2 | false, true | — | 1 |
| 3 | int | zigzag LEB128 varint | 1 |
| 4 | float | f64 LE | 1 |
| 5 | string | string | 1 |
| 6 | list | varint count, values | 1 |
| 7 | map | varint count, (key string, value) pairs | 1 |
| 8 | ref | kind string, id string | 1 |
| 9 | bytes | varint length, bytes | 1 |
| 10 | list of ints, packed | varint count, zigzag varints | 1 |
| 11 | stream | u32 LE byte length (`binary::STREAM_LEN_BYTES` = 4), then varint count and values with their own string table | 1 |
| 12 | float, 32-bit | f32 LE | 1 |
| 13 | list of floats, packed | varint count, f64 LE each | 1 |
| 14 | list of 32-bit floats, packed | varint count, f32 LE each | 1 |
| 15 | map of a shape | shape, then one value per key of the shape (no keys written) | 2 |
| 16 | list of maps of one shape | varint count, shape, then `count × width` values with no tag of their own between maps | 2 |
| 32 to 63 | string, from the table | none: the table entry is `tag - 32` | 2 |
| 128 to 255 | int, small | none: the int is `tag - 128 - 16` (-16 to 111) | 2 |

Tags 17 to 31, 64 to 127 are unused: `unknown_tag` in both versions. In a
version 1 document tags 15, 16, 32 to 63 and 128 to 255 are unknown too.

**Shapes** (version 2). A shape is the keys of a map, in order, written
once and named by number after that. A shape is written as a varint
`marker`: when odd, `marker >> 1` keys follow, each a string as above
(so the key text enters the string table, or is a reference to a table
entry that is already there), and the shape becomes the next shape number
(0, 1, 2, ...); when even, it names shape number `marker >> 1`, which must
exist (`bad_shape_index` otherwise). A shape has at least one key (a
written shape of no keys is `bad_length`; the empty map is tag 7, count
0) and no key twice (`duplicate_key`). The shape table belongs to the
document, or to a stream's body or a frame's tagged body, like the string
table. A map of tag 15 has as many values as its shape has keys; a list of
tag 16 has `count` maps of the same shape, and `count × width` values
follow back to back. Both are checked against the bytes left before
anything is allocated (every value is at least one byte).

**What a writer of a `Value` writes** (the canonical form, the same bytes
in every port; the conformance cases `binary_v2_is_encoded` pin it). It is
chosen from the value alone:

- an int in -16..=111 is its one-byte tag 128..=255; any other is tag 3;
- a float whose f64 is an `f32` exactly, bit for bit (`-0.0` and the
  infinities are; every NaN is not), is tag 12, else tag 4;
- a string that is already table entry 0..=31 is its tag 32..=63; any other
  is tag 5 and the string (a reference's kind and id, and keys, are always
  the string form, never a tag);
- a non-empty list of ints is tag 10; of floats tag 13, or tag 14 when every
  one narrows; a non-empty list whose items are all maps with the same
  keys in the same order (and at least one key) is tag 16; any other list
  is tag 6;
- a non-empty map is tag 15 with its shape, written the first time the keys
  (as table entries) are met and named after; the empty map is tag 7;
- strings enter the table in the order the writer meets them: a shape's
  keys in order, then each value's strings, depth first.

A writer of a typed value (`to_bytes`) may choose among the valid forms —
for instance writing the keys of an `Option` field that is absent as a
smaller shape — so its bytes need not equal the `Value` writer's; every
decoder reads every valid form to the same value.

Nothing is aligned. Decoding checks every count and length against the
bytes left and nesting against `MAX_DEPTH` (256; a shaped map is one
level, a list of records two: the list and each map): malformed input is
an error, never a panic or a huge allocation. A decoder fails loudly: it
never drops a byte or a value without saying so.

**Reading version 1.** A version 1 document is read with tags 0 to 14 only.
It is the same model: `binary::decode` of a version 1 document and of its
upgrade are equal values (`binary::upgrade`). The conformance suite keeps
the version 1 vectors as read vectors (`binary` in the manifest) beside
their version 2 forms (`binary_v2`), and says per case whether upgrading
writes exactly the version 2 bytes (`binary_upgrades`).

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
| list of ints | count, bit width (a byte), values bit-packed in lanes (see [Lanes](#lanes)) |
| list of `f64` | count, `f32`-exact flag (a byte), raw values (4 bytes each when the flag is set, else 8) |
| list of `f32` | count, raw values, 4 bytes each |
| tuple | as a list, each item its binary form as bytes |
| unit enum | varint variant index |
| `Result` | a map with one key: `ok` or `err` (`schema::RESULT_OK` / `RESULT_ERR`) |
| [`Atom`] | its number in this process's table |
| `Id<K>` (and `with::RefTo<K>`) | its id, as `K`'s id type packs (`@user:42` is one varint); the kind is the field's |
| `Ref` (any kind) | its kind and its id, as strings |
| anything else | its binary form, as bytes |

Reading rules a port must follow: every int in a frame is zigzag-coded,
including a numeric reference id; an absent `Option` reads as `null`, while a
field missing from a record that is shorter than the reader's reads as
absent (left out of the map, its default in a typed read); a bool byte above
1 is an error; the string table is read on demand; the root must consume its
whole body, so trailing bytes are an error. Ints are not range-checked against a [`Shape`]'s `min`/`max`
when read by shape alone.

A frame has no version byte. A value held "as its binary form, as bytes"
(a tuple's items, a tagged enum) is a value body with no header: version 2
tags, with a string table and a shape table of its own. Frames written
before version 2 hold version 1 bodies; those still read, because a
version 1 body is a valid version 2 body with the tags it used (tags 0 to
14 mean the same). Frames themselves are unchanged and need no upgrade
step: re-packing a value upgrades its bodies, and the conformance suite
keeps the frames written before as read vectors (`frames/`) beside what the
reference writes now (`frames_v2/`).

**Evolution rule:** fields are only ever appended. A record carries its
length, so a reader that knows fewer fields skips the rest and one that
knows more takes the defaults of the fields the record ends before: a field
with `default`, `None` for an `Option`; any other missing field makes the
frame unreadable. Variants are only ever appended too. What else may change,
and what each form does with it, is in [evolution.md](evolution.md).

`frame::to_frame` / `read_frame` for owned values; `#[data(view)]` on a
struct generates a zero-copy view (`frame::View`) over a frame's bytes.
`frame::read_shaped(bytes, &shape)` reads a frame by its [`Shape`] alone,
for a tool that has no code for the message (an editor's message
inspector).

`io::write_frame` / `read_frame` put messages on a byte stream (a socket, a
pipe): each a little-endian `u32` length (at most `io::MAX_FRAME`), then the
message's named binary form (`SDAT`, `to_bytes`). A stream ends cleanly only
between frames; one cut inside a length or a body is `UnexpectedEof`.

`io::FrameWriter` and `io::FrameReader` do the same from buffers they keep,
so a stream of messages allocates nothing per message once the buffers have
grown to the biggest one. A send writes the length and the body in one
`write_all`, then flushes; a frame over `MAX_FRAME` is refused
(`InvalidInput`) before anything is written.

- `FrameWriter::send_packed` writes the packed frame layout above (the
  length, then the frame): the zero-allocation path. The frame is packed
  in the writer's `Packer`, then copied once after the length.
- `FrameWriter::send` writes the named `SDAT` form, the same bytes
  `write_frame` writes, from a reused writer. It allocates nothing either
  once warm, except for a field written through a `Stream` (its body is
  built apart).
- `FrameReader::read_bytes` returns the frame's bytes, valid until the next
  read (the buffer grows as bytes arrive, not by the length a peer
  claims); `read::<T>()` decodes a named frame.

With the `tokio` feature, `io::asynchronous` reads and writes the same
frames over `AsyncRead`/`AsyncWrite` streams: `read_frame`,
`read_frame_bytes`, `write_frame`, `write_frame_bytes`, and the buffer-keeping
`FrameReader` and `FrameWriter`. The wire format, `MAX_FRAME` and the end
rules are the blocking ones, byte for byte (a stream cut inside a length
or a frame is `UnexpectedEof`, a length over `MAX_FRAME` is `InvalidData`
before anything is allocated for it); the async functions exist only
because a socket or a pipe waits, and encoding and decoding stay
synchronous. Cancellation: `FrameReader::read_bytes` and `read` are cancel
safe (what has arrived stays in the reader, so a dropped read loses
nothing, as long as the next call reads the same stream); the free
`read_*` functions keep nothing and are not; no write is cancel safe, since
a dropped write leaves a partial frame, so close the stream after one.

`binary::encode_into` and `to_bytes_into` append a whole document to a
caller's `Vec<u8>` (what `encode` and `to_bytes` build on); the buffer is
reused, the string index inside the call is not.

---

## Streams

`Stream<T>` is a list kept in the binary form: items written as they come,
read back one at a time when asked, with its own string table, so it can be
cut out of a document whole. For the big
ones: a table's rows, a picture's pixels, a recording, a log.

Decoding a stream checks its framing, not its items: the `u32` byte length
must fit the document, and the item count (the varint at the start of the
body) must fit the bytes of the body, one byte at least per item, so a
hostile count allocates nothing. An item is read when it is asked for:
`Stream::iter` yields `Result<T, ItemError>`, and an item that cannot be
read (a newer item type, a corrupt byte, a cut) is an `ItemError` with its
`index` and the `offset` of its first byte from the first item; the
iterator ends there. After the last counted item, bytes that are left are an
`ItemError` with `index` equal to the count. The wire format is the same as
in 0.3; only when the items are checked changed. The text form and frames
have no lazy path and read every item (an item that cannot be read rejects
the document, naming `items[i]`). Two streams are equal when their count and
their bytes are.

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
  turns it into a `ShapeDoc` — typed, itself a `Schema`, written
  `{ root, defs }` — that any tool can read: an editor builds property grids,
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

A format describes a type of its own (`"color"`, `"date"`, `"money"`,
`"duration"`) by implementing `NamedType` — its `NAME`, the `Stored` type
it is kept as, and the conversions — and `substrate_data::named_schema!(T)`
implements `Schema` from that. Its shape is `Shape::Named`, described
`{ kind: "named", name, of }`: a tool that knows the name shows its own
editor (a colour picker, a calendar), one that doesn't edits `of`. This
crate knows nothing of the names.

A named type is stored as `Stored` in every form (so `frame::read_shaped`
reads it as `of`); a stored value `from_stored` refuses is a problem at
its path. A list of a named type packs item by item (the default
`pack_list`), not as a packed list of numbers.

A reference's shape is `{ kind: "ref", of: [kinds], id: <id shape> }`.

### Resolving references to values

A reference can stand for a **value**: a word of a vocabulary, a localized
text, a row of a table. The format says where (a field written as a value
*or* as `@kind:id`); the tool says what the names mean.

- `Resolve::expand(&Ref, &Path) -> Result<Option<Value>, Unresolved>` — the
  value to read in the reference's place, `None` to keep it a reference
  (the default), `Err` for a name it won't take (SD0011, at the field's
  path). It is asked only for fields that opt in; `Resolve::reference`
  keeps its job (check or rewrite an id) for every other reference.
- `with::Expand<W>` — the field codec: a value that is a reference (a
  `Ref`, or a string spelling one) is expanded and `W` reads the result as
  if it had been written there; anything else, or no resolver, or a
  resolver that keeps it, is read by `W` as it stands. It composes with
  `Opt`, `List` and `Values`. A problem with the expanded value is reported
  at the reference's path; the reference is listed among the references
  met. An expanded value may itself hold fields that expand;
  `MAX_EXPANSION_DEPTH` (32) bounds a value that expands into itself.
- `Dictionary` — a ready resolver: values by kind and id, in the order
  added, written `{ <kind>: { <id>: <value>, … }, … }` (a `Schema`, so it is
  read from text, a value or binary like any document; notes keys are
  skipped). It checks references of the kinds it holds (a missing id is
  SD0011), expands them, and leaves other kinds alone. A string in it is
  *as spelled* like any `Value::Str`.

Which forms expand: the ones read **through a value**, with a resolver —
`from_value_with`, `from_doc_with`, `from_bytes_with` (the binary form
decoded borrowed, then read). `read_bytes` and `read_value` take no
resolver, and a frame is read with no `Reader` at all (`unpack`, views):
its schema is fixed on both sides, so a field holds its value, never a
name for one. Writing, `encode` and `pack` of an `Expand` field are the
inner codec's: the value, not the reference.

#### String tables

Repeated names already cost one number in the forms that carry a table:
the binary form interns every string (keys, kinds, ids and values) on the
way, a frame has one table at its end its strings index into, and a
`Stream` has its own. A process's `Atom` is a name kept once and sent in a
frame as its number (text and the tagged binary form carry its text). A
dictionary's words are ordinary strings: nothing beyond these tables is
needed to share them, and no new tag or frame field exists for it.

---

## Diagnostics

Every problem is a `Diagnostic`: a code, a severity, a message, and where
(file, path, line and column). Its code is a `Code`: `Code::Data(Problem)`
for this crate's (each `Problem` variant has its `code()`), or
`Code::Tool(Atom)` for a tool's own; both are written as the code's text.
Codes from this crate:

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
| SD0009 | binary data that won't decode (kinds: `bad_magic`, `bad_version`, `unexpected_end`, `unknown_tag`, `bad_varint`, `bad_length`, `bad_utf8`, `bad_string_index`, `bad_shape_index`, `bad_reference`, `duplicate_key`, `trailing_bytes`, `too_deep`) |
| SD0010 | a reference whose id isn't its kind's id type |
| SD0011 | a reference the [`Resolve`] doesn't resolve or expand |

Tools add their own codes; a `Diagnostic` is itself a `Schema`, so it
crosses a protocol like any other message.

[`Value`]: ../crates/substrate-data/src/model/value.rs
[`Schema`]: ../crates/substrate-data/src/schema/mod.rs
[`Shape`]: ../crates/substrate-data/src/schema/shape.rs
[`Resolve`]: ../crates/substrate-data/src/schema/reader/resolve.rs
[`Atom`]: ../crates/substrate-data/src/model/atom.rs

## Lanes

A list of `n` ints of `width` bits (1 to 32) is `n / 256` groups, then the
rest packed one after another, little-endian bits. A group is 256 numbers in
8 lanes of 32-bit words: number `j` goes to lane `j % 8`, the `j / 8`-th in
it; a lane's numbers are packed one after another into its words, and word
`k` of lane `l` is the group's `k * 8 + l`-th. Numbers wider than 32 bits go
one after another with no groups. The layout is what a vector of eight
32-bit lanes does naturally; an implementation may use any instruction set
(this crate picks SSE4.2, AVX2, AVX-512 or NEON at run time) or none, and
must write the same bytes.
