# Changing a type without breaking its data

What a derived type can change between versions while the data written by
one version is still read by the other. Everything here is proven by
`crates/substrate-data/tests/evolution`: each row of the matrix below is a
change to a type, with values written by one version and read by the other
in every form, and the test fails if this page and the result disagree
(`EVOLUTION_BLESS=1 cargo test -p substrate-data --test evolution`
rewrites the matrix after a deliberate change).

## The rules

1. **Append fields, never anything else.** A new field goes last, with
   `#[data(default)]` or as an `Option<T>`. Both directions then work in
   frames, and a newer reader of older data works in every form.
2. **Never remove, move, retype or insert before a field.** A frame has no
   field names and no field types, so a reader cannot tell that the bytes
   are another version's: it reads them as what its own schema says
   (`garbage` in the matrix), or fails when they happen not to fit. To retire a
   field, keep it in the type and stop using it (`skip` is not a way: a
   skipped field is not written and moves the ones after it).
3. **A required new field breaks old data.** A field with neither `default`
   nor `Option` makes every older frame, document and text unreadable
   (`MissingField`; in a frame, `None`).
4. **Append enum variants, never reorder or remove them.** A unit enum is a
   variant *number* in a frame; a tagged enum is a variant *name* everywhere.
   An older reader refuses a variant it does not know. For an open set,
   give a tagged enum an `other` variant: the older reader keeps what it
   cannot name as a `Value`.
5. **Numbers widen, they do not change kind.** An integer widens within its
   sign (`u8`, `u16`, `u32`, `u64`; `i32` to `i64`), and the narrower reader
   refuses a value that does not fit. Do not turn an integer into a float
   or a float of one width into the other: the named forms convert, a
   frame holds raw bytes and does not.
6. **A rename keeps its key.** Frames are positional, so a Rust-side rename
   is free there; the named forms are keyed, so keep the old key with
   `#[data(rename = "old")]`.

## What each form does

- **Frames** (`to_frame`, `io::FrameWriter::send_packed`) are positional.
  A record carries its byte length, so a reader that knows fewer fields
  skips the rest, and one that knows more takes defaults for the fields the
  record ends before (`default` fields their default, `Option` fields `None`;
  any other field makes the read `None`). Types are not checked: reading
  another type's bytes is only caught when the bytes do not fit.
- **Named forms** (the binary form `to_bytes`, text, values) are keyed by
  field name and are strict on purpose, so a typo in a text file is an
  error: a key the reader does not know is `UnknownField`, so an *older*
  reader does not read a *newer* document. A missing key is filled by
  `default` or `Option`, else `MissingField`. Order does not matter.
- **What a frame holds in its binary form** is the named form's strict
  reader, even inside a frame: a tagged enum, a tuple's items. A record
  that grows inside them is not skipped by its length: the older reader
  rejects it (the enum and tuple rows below).
- **`#[data(view)]`** over a frame: a field the writer did not have makes
  the view `None` (read the owned way then); extra trailing fields are
  skipped.
- **`frame::read_shaped`** over an older frame leaves the absent trailing
  fields out of the map; it does not default them.
- **A `Stream<T>`** keeps its items as bytes but reads each one once when
  the stream is decoded, so a stream of a newer item type read as the older
  one is rejected, naming the first item that cannot be read (`items[0]`),
  as text and frames do. Before this, the binary form kept the length and
  gave no readable items.
- **`u64` above `i64::MAX`** is carried as its bits (`format.md`), so
  an `i64` and a `u64` read each other's values by the same bits: a negative
  `i64` read as `u64` is a huge number, not an error.

## Matrix

Each cell is `frame / named forms` (the named forms are the binary form
and text; where they differ the cell says so). A row's two columns are the
two ways a change can meet old data:

- **New reader, old data**: data written before the change, read by the type
  after it.
- **Old reader, new data**: data written after the change, read by the type
  before it.

`reads` is the value the writer meant. `rejects` is an error (`None` for a frame), loud and
safe. **`garbage`** is a value that is not the one meant, with no error:
the cases to avoid, since nothing flags them.

<!-- matrix:start -->

#### Fields of a record

| Change | New reader, old data | Old reader, new data |
|---|---|---|
| append a field with `default` | reads / reads | reads / rejects |
| append an `Option` field | reads / reads | reads / rejects |
| append a required field (no `default`, not `Option`) | rejects / rejects | reads / rejects |
| insert a field with `default` before the last | rejects / reads | **garbage** / rejects |
| remove the last field | reads / rejects | rejects / rejects |
| remove the first field | **garbage** / rejects | rejects / rejects |
| swap two fields of one type | **garbage** / reads | **garbage** / reads |
| swap two fields of different types | rejects / reads | rejects / reads |
| rename a field in Rust, keeping its key (`rename`) | reads / reads | reads / reads |
| rename a field and its key | reads / rejects | reads / rejects |

#### Numbers

| Change | New reader, old data | Old reader, new data |
|---|---|---|
| `u8` to `u32`, values within `u8` | reads / reads | reads / reads |
| `u8` to `u32`, the new writer's value over 255 | reads / reads | rejects / rejects |
| `i32` to `i64`, a negative value | reads / reads | reads / reads |
| `u32` to `i32`, values outside the other's range | rejects / rejects | rejects / rejects |
| `i64` to `u64`, a negative value (a `u64` above `i64::MAX` is its bits as an `i64`) | **garbage** / **garbage** | **garbage** / **garbage** |
| `u32` to `f64` | rejects / reads | **garbage** / reads |
| `f32` to `f64` | rejects / reads | **garbage** / reads |
| `f64` to `f32`, a value that is not an `f32` | **garbage** / **garbage** | rejects / reads |
| `Vec<u8>` to `Vec<u32>` | reads / reads | reads / reads |
| `Vec<f32>` to `Vec<f64>` | rejects / reads | **garbage** / reads |

#### Optional and required

| Change | New reader, old data | Old reader, new data |
|---|---|---|
| `u32` to `Option<u32>` | rejects / reads | rejects / reads |
| `Option<u32>` to `u32`, the old value `Some` | rejects / reads | rejects / reads |
| `Option<u32>` to `u32`, the old value `None` | **garbage** / rejects | rejects / reads |
| `String` to `Option<String>` | **garbage** / reads | rejects / reads |

#### Enums

| Change | New reader, old data | Old reader, new data |
|---|---|---|
| unit enum: append a variant | reads / reads | rejects / rejects |
| unit enum: insert a variant before the last | **garbage** / reads | **garbage** / rejects |
| unit enum: swap two variants | **garbage** / reads | **garbage** / reads |
| unit enum: remove the last variant | rejects / rejects | reads / reads |
| tagged enum: append a variant | reads / reads | rejects / rejects |
| tagged enum with an `other` variant (old): append a variant | reads / reads | reads / reads |
| tagged enum: swap two variants | reads / reads | reads / reads |
| tagged enum: a variant gains a field with `default` | reads / reads | rejects / rejects |
| tagged enum: a variant's record gains a field with `default` | reads / reads | rejects / rejects |

#### A record that grows, held in another type

| Change | New reader, old data | Old reader, new data |
|---|---|---|
| a record in a field | reads / reads | reads / rejects |
| a record in an `Option` | reads / reads | reads / rejects |
| a record in a `Box` | reads / reads | reads / rejects |
| records in an array | reads / reads | reads / rejects |
| records in a `Vec` | reads / reads | reads / rejects |
| a record in a tuple | reads / reads | rejects / rejects |
| records in a `Stream` | reads / reads | reads / rejects |

<!-- matrix:end -->

## Telling versions apart

A frame cannot say which version wrote it. When readers of different
versions share a wire, put a version in the message yourself, as the first
field of the root record (the one field that never moves), and refuse
what is newer than you know before reading further.
