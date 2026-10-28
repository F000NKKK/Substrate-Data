# Changelog

All notable changes are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the crates follow
[Semantic Versioning](https://semver.org/). While the version is `0.x`, a
breaking change to the Rust API raises the minor version. The text grammar,
the `SDAT` binary tags and the frame layout in [docs/format.md](docs/format.md)
only grow: a tag is never renumbered and a frame field is never reordered.

## [Unreleased]

## [0.3.1] - 2026-10-28

### Changed

- The Java port targets Java 17 (`maven.compiler.release` 17): it runs on
  17, 21 and 25 alike. Pattern switches are `instanceof` chains; all 325
  conformance cases pass on JDK 17.

### Added

- Java port: `Binary.encodeBody`, a value without a header with a string
  table of its own — what `Binary.decodeBody` reads (a value a frame holds
  but doesn't pack). It writes version 2, as every writer does: there is no
  version 1 writer.

## [0.3.0] - 2026-10-27

0.3 is the performance and compactness release. The headline changes:

- **SDAT version 2** (breaking, binary form): readers accept versions 1 and 2,
  writers always write 2. Documents are 12 to 55 % smaller and decode up to
  2.5x faster; every version 1 document converts without loss
  (`binary::upgrade`).
- **Text form** parses in about half the instructions and writes up to 2.4x
  faster; typed `to_text`/`from_text` no longer build a `Value`.
- **Benchmarks** for every public path, a comparison with serde_json,
  ciborium, rmp-serde, postcard and bincode, and instruction-count baselines.
- **Ports** to TypeScript and Java (`ports/`), kept in step by the
  conformance vectors.
- **Async frame io** (feature `tokio`), **schema-evolution** documentation
  proven by tests, and AVX-512/AVX2 dispatch tuned from measurements.

Migration from 0.2.x, in one place:

1. Anything you write is now SDAT version 2. Upgrade every reader of that data
   to 0.3 (or to a port from this release) before you write version 2 with it;
   0.2 readers cannot read it. Old files still read; `binary::upgrade(bytes)`
   rewrites one as version 2.
2. A binary `Stream<T>` whose items cannot be read is now an error at decode
   instead of an empty stream.
3. `Schema` and `With` gain the provided method `write_text`; nothing to do
   unless you implement both by hand and want the fast path.
4. The error vector `binary_version_2` of the conformance suite is now
   `binary_version_3` (ports and other readers of the vectors).

Details, with measured numbers, follow.

### Fixed

- Binary form: a `Stream<T>` whose items `T` cannot read (an older reader
  given a newer item type, or a corrupt item) was accepted with its length
  and no readable items: `to_vec()` was empty and `iter()` stopped at the
  first item it could not read. Decoding now reads each item once and
  rejects the stream, as the text form and frames already did;
  `read_bytes` names the first item that fails (`items[0]`). A stream body
  with bytes left after its counted items is rejected too. The wire format
  is unchanged. Migration: a reader that relied on getting an empty stream
  from a newer writer now gets an error; read such data with the newer item
  type (see [docs/evolution.md](docs/evolution.md)). `Stream::iter` and
  `to_vec` are unchanged in signature, and cannot fail on a decoded stream.
  Decoding a stream now costs one pass over its items.

### Added

- Benchmarks for every public path (one criterion group per area), a shared
  neutral corpus, `benches/compare.rs` against serde_json, ciborium,
  rmp-serde, postcard and bincode, an iai-callgrind instruction-count suite
  and `cargo run --example sizes`; see [docs/performance.md](docs/performance.md).
- `LANES_WIDTHS` selects the widths the `lanes` bench measures.
- `LANES_GROUPS` sets the number of groups a `lanes` bench run packs and
  unpacks (64 by default), to see where AVX-512 hands a tail to AVX2.
- `ports/typescript` and `ports/java`: ports of the value model, text form,
  binary form, frame reading and io streams, each passing every conformance
  vector (CI jobs `port-typescript`, `port-java`).
- Conformance: the `duplicate_key` binary fault class, with two vectors.

### Added (async io, feature `tokio`)

- `io::asynchronous` (feature `tokio`, off by default; the dependency is
  tokio's `io-util` alone): `FrameReader`, `FrameWriter`, `read_frame`,
  `read_frame_bytes`, `write_frame`, `write_frame_bytes` over
  `AsyncRead`/`AsyncWrite`. Same wire format, `MAX_FRAME` and end rules as
  the blocking functions (tested on the same bytes); a length is checked
  before anything is allocated and the buffer grows only with the bytes
  that arrive. `FrameReader` reads are cancel safe; writes are not (see
  the module docs).
- `benches/io_async.rs` (needs the feature); see
  [docs/performance.md](docs/performance.md).

### Added (schema evolution)

- [docs/evolution.md](docs/evolution.md): what a derived type may change
  between versions, per form, as a matrix written from the
  `tests/evolution` rows (the test fails when the page is stale).
  Documents, with tests, that frames skip appended fields but do not detect
  a reorder or a retype, that the named forms refuse unknown fields, and that
  records inside tagged enums and tuples are not skipped by length.

### Added (text form)

- `text::parse_ref(text, &arena)`: parses straight to a `ValueRef` (no
  `Spans`, strings borrowed from the text).
- `text::Writer`: the text form written piece by piece, compact or pretty,
  without a `Value` (`map(n)`, `key`, `list(n)`, scalars, `reference`,
  `bytes`; containers close by themselves).
- `Schema::write_text` and `With::write_text` (default: through `write`);
  derived structs, newtypes and unit enums, the plain types, `Vec`, `Option`,
  maps and the `with` codecs write straight. `to_text` uses it.

### Changed (text form)

- Text performance, same grammar and byte-identical output (instruction
  counts, millions, `--all-features`, large samples): parse `config`
  51.2 to 29.0, `records` 48.5 to 24.0, `deep` 68.3 to 35.7; write `numbers`
  44.1 to 18.1, `refs` 24.4 to 13.8, `blobs` 10.5 to 3.4; typed `to_text`
  (`docs_large`) 54.6 to 13.9, `from_text` 82.1 to 30.8, `numbers_large`
  49.6 to 18.6 and 43.5 to 27.0.
- Floats are spelled from the `zmij` crate's shortest digits (new
  dependency, no dependencies of its own) in the same `{:?}` spelling.
- Breaking: `Schema` and `With` gain a provided method (`write_text`), so
  nothing to migrate unless a type implements both by hand and wants the fast
  path. `text::parser` internals moved; `MAX_DEPTH`, `parse`, `parse_value`
  are unchanged.
- `Map`'s index uses a seeded multiply-rotate hasher instead of SipHash
  (hash-flooding is still not predictable: the seed is random per table);
  keys of a parsed `Value` are shared between maps.

### Changed (SDAT version 2)

Breaking: the binary form is now version 2, and the Rust API gains a version
rule. Measured on the large corpus samples (iai-callgrind instruction counts,
millions, `v1` baseline against this change; bytes from
`cargo run --example sizes`); see [docs/performance.md](docs/performance.md).

- **Writing is always version 2; reading accepts versions 1 and 2.** There is
  no version 1 writer and no mode that keeps a document's version: reading a
  version 1 document and writing it upgrades it, and `binary::upgrade(bytes)`
  does exactly that. Every version 1 value has a version 2 form (property
  test over every value type, plus every conformance vector); an unknown
  version byte is `bad_version`, and a tag the version doesn't have is
  `unknown_tag`. Migration: nothing to convert; old files and messages still
  read, anything written now is version 2 and version 1 readers (the
  published 0.2 crate, ports before this change) cannot read it, so upgrade
  readers first.
- New tags (the version 1 tags 0 to 14 are unchanged): 15 a map of a shape
  (its keys listed once, then named by number), 16 a list of maps of one
  shape (the keys once, no tag between maps), 32 to 63 a string already in the
  table (no payload), 128 to 255 an int from -16 to 111 (no payload).
- `encode(&Value)` writes lists of ints (tag 10) and floats (tags 13 and 14),
  floats that are `f32` exactly in four bytes, and records; the form is chosen
  from the value alone, so ports write the same bytes.
- Bytes (large samples): `Value` trees config 186015 to 153160 (-18 %),
  records 199200 to 159215 (-20 %), numbers 275815 to 185815 (-33 %), deep
  144385 to 64470 (-55 %), refs 122991 to 89750 (-27 %); typed `to_bytes`
  users 252167 to 201322 (-20 %), docs 199200 to 165014 (-17 %), files 224721
  to 196927 (-12 %).
- Instructions (millions): `encode(&Value)` records 11.0 to 6.2, numbers 3.3 to
  1.9, refs 16.4 to 10.9, but config 11.9 to 13.0 and deep 11.4 to 12.6 (+9 and
  +11 %: a shape lookup per map where nearly every map has other keys, fixed
  below);
  `decode` records 19.1 to 11.3, numbers 5.4 to 2.1, deep 27.4 to 18.9, config
  22.8 to 20.1, refs 25.7 to 21.6; typed `to_bytes` docs 9.0 to 4.6, `to_frame`
  docs 5.1 to 4.6 (numbers 0.56 to 0.62, +11 %).
- Frames keep their layout (no version byte; fields only appended). A tagged
  value inside a frame is a version 2 body now; frames written before still
  read, and the conformance suite keeps them as read vectors (`frames/`) beside
  the version 2 ones (`frames_v2/`). Re-packing upgrades the bodies.
- `binary::Writer::record` / `record_masked` (a map by its keys) are new;
  `DecodeError` has the class `bad_shape_index`; `Map::keys` is `Clone`.
- Conformance: the version 1 vectors are kept as read vectors, new
  `binary_v2*`, `binary_upgrades` and frame `bytes_v2*` fields and the
  `binary_v2` and `frames_v2` directories; the `binary_version_2` error vector
  is replaced by `binary_version_3` (version 2 is a version now). The
  TypeScript and Java ports read version 1, write version 2 and pass every
  vector.

### Changed (performance after SDAT version 2)

Same bytes in every form (the encoded corpus is identical byte for byte, the
conformance vectors are untouched, so the TypeScript and Java ports need
nothing). Instruction counts, millions, large samples (`patch::format` on the
medium ones), iai-callgrind `--cache-sim=no`, main at 2550613 against this
change; see [docs/performance.md](docs/performance.md), "Tuning after version
2".

- `binary::encode(&Value)` config 12.94 to 9.25 (-28 %) and deep 12.44 to 9.14
  (-27 %), which were 9 and 11 % above version 1 and are now 22 and 20 % below
  it: a writer compares a map's keys with those of the last four shapes it
  used before looking anything up, and a shape that is new is found in one
  pass instead of two. records and refs are unchanged (+1 % and 0 %).
- Typed `to_frame` numbers 0.70 to 0.49 (-30 %, version 1 0.56) and typed
  `to_bytes` numbers 1.25 to 1.04 (-17 %): packed floats are written without
  zeroing the buffer first.
- `binary::decode` refs 21.94 to 15.76 (-28 %), config 20.15 to 17.57 (-13 %),
  numbers 2.06 to 1.90 (-8 %); typed `read_bytes` numbers 1.83 to 1.66, docs
  9.20 to 8.90: a reference's kind is checked once per string of the table,
  `RefKey::is_valid` has an ASCII path, `atom::local` hashes with the crate's
  seeded multiply-rotate hasher, a decoded map checks duplicate keys against
  its own index (no `HashSet<String>`), and a one-byte varint is read inline.
- `patch::format` 1.06 to 0.22 (config), 2.19 to 0.38 (deep), 1.21 to 0.27
  (records), 1.33 to 0.40 (refs), 1.27 to 0.61 (numbers): one `String` instead
  of one per node (byte-identical output); `patch::edit` refs 2.15 to 1.38.
- Frame unpack is unchanged: its `malloc` share is the owned `String` of each
  string field, not the unpacker.

### Changed

- Lanes: the AVX-512 against AVX2 choice is now made per operation and width
  from measurements (docs/performance.md, "Lanes: AVX-512 against AVX2"):
  packing 32-bit groups stays on AVX2 (AVX-512 is 1.2 to 1.6x slower there),
  unpacking 32-bit groups uses AVX-512 for runs of up to 24 groups (about 2x
  faster) and AVX2 beyond. Every other width, and every group count of two or
  more, stays on AVX-512. The bytes written and read do not change.
- `docs/format.md` states what the ports found unspecified: the exact float
  text spelling, canonical base64 for `@bytes:`, and the frame reading rules.

## [0.2.1] - 2026-10-27

### Fixed

- An integer outside its type's range (`300` as `i8`, `-1` as `u8`) is
  reported as `SD0007` (number out of range), as the range checks of fields
  already did; it was `SD0000`.
- A frame list of `u64` or `usize` holding a value above `i64::MAX` could not
  be read as a `Value` (`frame::read_shaped` gave `None`); it now reads as the
  bits `write` makes (`u64::MAX` is `Int(-1)`), and the typed read was already
  right.
- `Writer::float` documented a narrowing it never did; the docs say what it
  writes (eight bytes).

### Changed

- `docs/format.md` no longer carries a section about where the crate came
  from, and says how a `u64` above `i64::MAX` is carried in an int.

## [0.2.0] - 2026-10-27

### Added

- `KindName` and `RefKey` read as `&str` (`Deref`, `AsRef<str>`), a
  `RefKey` borrows as `str` (map lookups by text), and both compare with
  `String`.
- `io::FrameWriter` (`send` for the named form, `send_packed` for the frame
  layout) and `io::FrameReader`: frames sent and read from reusable buffers,
  one `write_all` per frame, no allocation per message once warm.
- `binary::encode_into` and `to_bytes_into`: append the binary form to a
  caller's buffer.
- Reference expansion: `Resolve::expand` (a reference replaced by a value;
  default keeps it), `with::Expand<W>` (a field written as a value or as a
  reference), `Reader::expand_ref`, `Expansion`, `MAX_EXPANSION_DEPTH`, and
  `from_bytes_with` (the binary form read through a resolver). The frame
  forms don't expand: a frame has no resolver, its fields hold values.
- `schema::Dictionary`: values by kind and id, a `Schema` and a `Resolve`
  that checks and expands the references of the kinds it holds.
- `conformance/`: language-neutral golden vectors (a JSON manifest and the
  raw bytes) for ports in other languages, and a test that fails when the
  implementation drifts from them (`CONFORMANCE_BLESS=1` regenerates).
- `simd-utf8` feature: SIMD UTF-8 validation (`simdutf8`) for source files,
  the binary form and frames; results and errors are identical to the
  standard library's.
- The text parser's string, comment and line searches use `memchr`
  (runtime-dispatched SIMD); behaviour, errors and positions are unchanged.
- Bit-packing of integer lists runs on the best instruction set the
  processor has, chosen once at start: SSE4.2, AVX2 or AVX-512 on x86-64, NEON
  on AArch64, else portable code. `frame::lanes::Backend` names them
  (`active`, `available`, `pack_groups`, `unpack_groups`). The bytes are the
  same on every backend; kernels are specialised per bit width.
- `binary::NEW_STRING`, `binary::STREAM_LEN_BYTES`, `schema::RESULT_OK` and
  `schema::RESULT_ERR` are public, for readers in other languages.

### Changed

- `kind!` takes a kind's name as a string literal, a path to a `&'static str`
  constant (`Mesh = ids::MESH`) or a `{ block }`; the name is checked where
  the kind is declared, not where it is first used.
- Dependencies: `base64` 0.23, `toml` 1, `syn` 3, `criterion` 0.8.
- `docs/format.md` names the public constants and the `Result` keys.

### Fixed

- `frame::Packer` no longer allocates to write the length of a record or
  list over 127 bytes.
- The binary decoders refuse a map with a repeated key (`binary::decode`,
  `decode_ref`), as the text parser already did; before, both entries were kept.
- A stream cut inside a frame's length is an error, not a clean end
  (`io::read_frame_bytes`).
- The benches no longer carry a `missing_docs` expectation that
  `criterion` 0.8 makes unfulfilled (it failed `clippy -D warnings`).

## [0.1.0] - 2026-10-27

First release of `substrate-data` and `substrate-data-derive`.

### Added

- One value model (`Value`: null, bool, int, float, string, list, map, typed
  reference, bytes) with typed strings (`@kind:id`, `@bytes:…`, `@@` escape).
- Text form: JSON with comments, trailing commas and unquoted keys, a span
  for every value, and `patch` to edit a document in place keeping comments
  and layout.
- Binary form `SDAT` v1 (tags 0–14, interned strings), packed frames whose
  fields are only ever appended, and streams.
- `Schema` and `#[derive(Schema)]` (`#[data(...)]` options) with `Shape`
  descriptions an editor can build property grids from, typed references
  (`Kind`, `Id<K>`, `Resolve`) and diagnostics (`SD0000`–`SD0011`) that carry
  line, column and path.
- Optional `toml` and `json` features.

[Unreleased]: https://github.com/F000NKKK/Substrate-Data/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/F000NKKK/Substrate-Data/releases/tag/v0.1.0
