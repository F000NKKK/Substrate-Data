# Changelog

All notable changes are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the crates follow
[Semantic Versioning](https://semver.org/). While the version is `0.x`, a
breaking change to the Rust API raises the minor version. The text grammar,
the `SDAT` binary tags and the frame layout in [docs/format.md](docs/format.md)
only grow: a tag is never renumbered and a frame field is never reordered.

## [Unreleased]

### Added

- `conformance/`: language-neutral golden vectors (a JSON manifest and the
  raw bytes) for ports in other languages, and a test that fails when the
  implementation drifts from them (`CONFORMANCE_BLESS=1` regenerates).
- `binary::NEW_STRING`, `binary::STREAM_LEN_BYTES`, `schema::RESULT_OK` and
  `schema::RESULT_ERR` are public, for readers in other languages.

### Changed

- Dependencies: `base64` 0.23, `toml` 1, `syn` 3, `criterion` 0.8.
- `docs/format.md` names the public constants and the `Result` keys.

### Fixed

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
