# Changelog

All notable changes are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the crates follow
[Semantic Versioning](https://semver.org/). While the version is `0.x`, a
breaking change to the Rust API raises the minor version. The text grammar,
the `SDAT` binary tags and the frame layout in [docs/format.md](docs/format.md)
only grow: a tag is never renumbered and a frame field is never reordered.

## [Unreleased]

### Added

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
