# Conformance vectors

Golden data for ports of Substrate Data to other languages (Java,
TypeScript, ...): the exact bytes the Rust reference reads and writes, for
every value tag, the string table, streams, packed lists, frames, the frame
stream and the malformed inputs it refuses. A port that passes these agrees
with the reference on the wire, without reading Rust. The format itself is
specified in [../docs/format.md](../docs/format.md).

Domain-neutral: the cases use only users, docs, files and projects.

## Files

| Path | What |
|---|---|
| `cases.json` | the manifest: an array of cases, one object each |
| `binary/<name>.sdat` | the case's binary form (an `SDAT` document), raw |
| `frames/<name>.frame` | the case's frame, raw |
| `io/<name>.stream` | the case's length-prefixed frame stream, raw |

Hex strings in the manifest are lower case, no separators. Bytes longer
than 256 are only in their file (the manifest gives `..._len` and
`..._file` for every one). Everything is generated; do not edit by hand.

## A case

```jsonc
{
  "name": "int_64",                  // unique; the raw files are named by it
  "description": "...",              // what it shows
  "text": "64",                      // the text form, when there is one
  "text_is_written": true,           // the reference writes exactly this text for `value`
  "value": {"t": "int", "v": "64"},  // the expected model value (below)
  "binary_is_encoded": true,         // the reference writes exactly these bytes for `value`
  "binary": "5344415401038001", "binary_len": 8, "binary_file": "binary/int_64.sdat",
  "frame": {...}, "io": {...}, "error": {...}  // see below; any subset
}
```

A case has any subset of the forms; what a port checks is listed per form.

### Values

A model value is JSON with a type tag `t`, so ints, floats, strings, typed
strings and bytes are never confused:

| `t` | Rest | Model value |
|---|---|---|
| `null` | | null |
| `bool` | `v`: true/false | bool |
| `int` | `v`: decimal string | i64 (a string: JSON numbers lose bits past 2^53) |
| `ints` | `v`: array of JSON numbers | a non-empty list of ints, each below 2^53 (a shorthand) |
| `float` | `bits`: 16 hex digits | f64 by its IEEE 754 bits (also `v`: a decimal spelling, for people, when finite) |
| `str` | `v` | a string *as spelled*: plain text starting with `@` is stored with it doubled (`"@@x"`); `"@file:x"` stays a string here |
| `list` | `v`: array of values | list |
| `map` | `v`: array of `[key, value]` pairs, **in order** | map; keys unique, order is part of the value |
| `ref` | `kind`, `id` | a reference |
| `bytes` | `hex` | bytes |

Compare floats by their bits: `-0.0` is not `0.0`, and a NaN equals itself.
A map is an ordered list of pairs; two maps with the same pairs in another
order are different values.

## What to check, per form

**`text`** (a value in text form):

1. Parse `text`; the result must be `value` with every `ref` and `bytes`
   replaced by its string spelling (`@kind:id`, `@bytes:<base64>`). The
   text parser keeps typed values as the strings they are written as; a
   schema gives them their type.
2. When `text_is_written` is true, writing `value` as compact text must give
   `text` exactly (floats in the shortest spelling that reads back as the
   same f64 and as a float: `1.0`, `1e16`, `-0.0`).

Text-only cases (the `syntax_*` and `text_*` cases without binary) only
read: they pin the grammar (comments, trailing commas, unquoted keys, a
byte order mark, escapes, an integer past i64 reads as a float).

**`binary`** (the bytes of `binary_file`):

1. Decode them; the result must be `value`.
2. When `binary_is_encoded` is true, encoding `value` must give the bytes
   exactly. When it is false the bytes come from a typed writer (a packed
   list tag, a stream): a decoder must read them to `value`, a writer is
   not required to choose that form.
3. A case with `binary` and no `value` only has to decode and encode back to
   the same bytes (`binary_depth_256`).

A writer that has a list of f64 uses tag 14 (f32 each) when every item
survives f64 -> f32 -> f64 bit for bit, else tag 13. An empty list is tag 14.

**`frame`** (a value packed for a message whose schema both sides know):

`frame.schema` names a schema of this suite and `frame.shape` is its
description as compact text (`Shape::describe`: `{ root, defs }`), which is
all a port needs to read the frame by its shape. Read `frame.bytes` (hex or
`bytes_file`) by that shape; the result must be `frame.value`. When
`also_read_by` is present, the same bytes read by each listed schema must
give that entry's `value` (a record that grew: a reader that knows fewer
fields skips the rest; one that knows more finds the missing trailing fields
absent). Reading a frame by shape gives plain text as the model spells it
(a leading `@` doubled). A frame's `value` can differ from the case's
`value`: the case's `value` is its binary document. A port that packs frames
must produce `bytes` from the typed value; the case's `text` and `binary`
are that value's other forms.

Rules the vectors pin that are easy to get wrong:

- the table length is a varint written **backwards** from the frame's end;
- a record's length is a varint of one byte (<= 127) or more;
- an `f64` list has a flag byte (1: every item is an f32 exactly);
- a list of ints has a count, a width byte (1..=64) and bit-packed lanes: for
  widths up to 32 a full group of 256 numbers is packed in 8 lanes of
  32-bit words, the rest (and every list of wider numbers) one after another,
  little-endian bits; a signed list is zigzagged first (`frame_ints_*`);
- a tagged enum is held as its binary form (own string table, no header).

**`io`** (a stream of messages):

Each message is a little-endian u32 length, then that many bytes (a binary
document). Reading `bytes` frame by frame must give `io.messages` (hex) and
then end cleanly: no byte left where the next length would start.

**`error`** (a malformed input; `error.form`):

| `form` | Fields | A port must |
|---|---|---|
| `text` | `input`, `line`, `col`, `message` | fail to parse, at that 1-based line and column; code SD0001 |
| `text_schema` | `schema`, `input`, `diagnostics` | parse, then report the diagnostics (`code`, `line`, `col`, `path`) in file order |
| `binary` | `input` (hex), `kind`, `offset`, `message`, `schema_code` | refuse the bytes (never panic, never allocate by a lying length); SD0009 when read as a schema |
| `frame` | `schema`, `shape`, `input` (hex) | read nothing |
| `io` | `input` (hex), `frames_before`, `kind` | read `frames_before` frames, then fail with `kind` |

`message` is the reference's wording and `offset` its position: both are
informational (a port may word an error as it likes; an offset inside a
stream's body is relative to that body). What must agree is the class:

| `binary.kind` | Meaning |
|---|---|
| `bad_magic` | no `SDAT` header |
| `bad_version` | a version byte other than 1 |
| `unexpected_end` | input ends where a byte is needed |
| `unknown_tag` | a tag byte not in 0..=14 |
| `bad_varint` | a varint cut short, or longer than 10 bytes, or a 10th byte above 1 |
| `bad_length` | a count or length past the bytes left (checked before any allocation) |
| `bad_utf8` | a string that is not UTF-8 |
| `bad_string_index` | an index not yet in the string table |
| `bad_reference` | a reference whose kind or id is not valid |
| `trailing_bytes` | bytes after the root value (or after a stream's items) |
| `too_deep` | nesting beyond `MAX_DEPTH` (256) |

| `io.kind` | Meaning |
|---|---|
| `unexpected_eof` | the stream ends inside a length or a frame |
| `invalid_data` | a length above `MAX_FRAME` (268435456 bytes) |

Diagnostic codes covered: SD0001 .. SD0008, SD0009, SD0010. (SD0000 and
SD0011, which needs a resolver, are not.)

## Versioning

Vectors only grow. A new case may be added at any time; an existing case,
and every byte of its files, is never changed: **a changed byte is a format
break**, and needs a new binary version, not a new vector. Removing a case
is a format break too.

## Regenerating

The Rust test `crates/substrate-data/tests/conformance` builds every case
from the reference implementation and fails when a file differs, is missing
or is extra. To write the files on purpose (a new case, a new
version):

```sh
CONFORMANCE_BLESS=1 cargo test -p substrate-data --test conformance
```

then review the diff: for an existing case it must be empty.
