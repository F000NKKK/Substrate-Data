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
| `binary/<name>.sdat` | the case's version 1 binary form (an `SDAT` document), raw: read-only legacy |
| `binary_v2/<name>.sdat` | the case's version 2 binary form, raw |
| `frames/<name>.frame` | the case's frame, raw (what was written before version 2) |
| `frames_v2/<name>.frame` | the case's frame as the reference writes it now, when that differs from `frames/` |
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
  "binary_is_encoded": true,         // the old (version 1) writer wrote exactly these bytes for `value`
  "binary": "5344415401038001", "binary_len": 8, "binary_file": "binary/int_64.sdat",  // version 1: READ ONLY
  "binary_v2_is_encoded": true,      // the reference writes exactly these bytes for `value`
  "binary_v2": "5344415402c0", "binary_v2_len": 6, "binary_v2_file": "binary_v2/int_64.sdat",
  "binary_upgrades": true,           // reading `binary` and writing gives exactly `binary_v2`
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

**Version 1 and version 2.** SDAT version 2 is the only version written; a
reader reads versions 1 and 2. The vectors of version 1 (`binary`, the
files in `binary/`) are never changed and are kept as **read vectors**: a
port decodes them and does not write them (there is no version 1 writer,
`binary_is_encoded` only says what the old writer wrote). The version 2
vectors (`binary_v2`, `binary_v2_file` in `binary_v2/`) are what the
reference writes now. A case of both forms is an upgrade case.

**`binary`** (the bytes of `binary_file`, version 1):

1. Check byte 4 is `1`. Decode them; the result must be `value`.
2. Never write them back. (`binary_is_encoded` is the old writer's.)

**`binary_v2`** (the bytes of `binary_v2_file`, version 2):

1. Check byte 4 is `2`. Decode them; the result must be `value`.
2. When `binary_v2_is_encoded` is true, encoding `value` must give the bytes
   exactly (the canonical form, see `docs/format.md`: shapes, records,
   inline ints, string tags, packed lists, `f32` narrowing). When it is
   false the bytes come from a typed writer (a stream, a masked record): a
   decoder must read them to `value`, a writer is not required to choose
   that form.
3. A case with `binary_v2` and no `value` only has to decode and encode back
   to the same bytes (`binary_v2_depth_256_shaped_maps`).

**`binary_upgrades`** (a case with `binary` and `binary_v2`): decode `binary`
and encode the result. The bytes are exactly `binary_v2` when
`binary_upgrades` is true; when false they are another valid version 2
document (the version 2 bytes came from a typed writer, a stream or a masked
record) that must decode to the same value. The upgrade never loses a
value: this is the lossless rule of the format, and the Rust suite also
checks it over generated values of every type.

Every `binary` error input is read at version 1 (the original vectors) and
the same malformed documents at version 2 are in the `binary_v2_*` error
cases. An unknown version byte is `bad_version`: both the reference and a
port refuse it and read nothing.

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
- a tagged enum is held as its binary form (own string table, no header):
  version 1 in `frames/`, version 2 (shapes, inline ints) in `frames_v2/`;
  a frame has no version byte and its bodies read either way (`bytes_v2` is
  present when the reference now writes other bytes; read both).

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
| `bad_version` | a version byte other than 1 or 2 |
| `unexpected_end` | input ends where a byte is needed |
| `unknown_tag` | a tag byte the document's version doesn't have (version 1: 0..=14; version 2: 0..=16, 32..=63, 128..=255) |
| `bad_varint` | a varint cut short, or longer than 10 bytes, or a 10th byte above 1 |
| `bad_length` | a count or length past the bytes left (checked before any allocation) |
| `bad_utf8` | a string that is not UTF-8 |
| `bad_string_index` | an index (or a string tag) not yet in the string table |
| `bad_shape_index` | a shape number not yet defined (version 2) |
| `bad_reference` | a reference whose kind or id is not valid |
| `duplicate_key` | a map with a key that is already in it (compared by text, however the string was written) |
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
is a format break too. Version 2 added fields beside the version 1 ones
(`binary_v2*`, `binary_upgrades`, `bytes_v2*`) and left every version 1
file as it was. The one case that left is `binary_version_2` (a document of
version 2 read as an error): version 2 is a version now, so that input is
`binary_version_3`'s sibling no more; the manifest has `binary_version_3`
in its place, and this is the version change itself.

## Regenerating

The Rust test `crates/substrate-data/tests/conformance` builds every case
from the reference implementation and fails when a file differs, is missing
or is extra. To write the files on purpose (a new case, a new
version):

```sh
CONFORMANCE_BLESS=1 cargo test -p substrate-data --test conformance
```

then review the diff: for an existing case it must be empty (version 1 files
are never rewritten: the generator keeps what is on disk as the legacy form
and writes only the version 2 bytes beside it).
