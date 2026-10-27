# Changelog

All notable changes are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the crates follow
[Semantic Versioning](https://semver.org/). While the version is `0.x`, a
breaking change to the Rust API raises the minor version. The text grammar,
the `SDAT` binary tags and the frame layout in [docs/format.md](docs/format.md)
only grow: a tag is never renumbered and a frame field is never reordered.

## [Unreleased]

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
