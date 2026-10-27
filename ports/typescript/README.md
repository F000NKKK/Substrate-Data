# Substrate Data for TypeScript

A TypeScript port of [Substrate Data](../../README.md): the value model, the
text form, the binary form (`SDAT` version 2, version 1 still read), frames read by their shape, and
length-prefixed frame streams. It has no runtime dependencies, is strict
TypeScript (ESM, Node 22 or later) and is checked against the language-neutral
[conformance vectors](../../conformance/README.md): the exact bytes the Rust
reference reads and writes. The format itself is specified in
[docs/format.md](../../docs/format.md).

## Install and build

```sh
cd ports/typescript
npm ci
npm run build
```

`typescript` and `@types/node` are the only (development) dependencies.

## Usage

```ts
import {
  decodeBinary, encodeBinary, int, list, map, parseText, str, writeText,
} from 'substrate-data';

const doc = map([
  ['name', str('Release 1.0')],
  ['files', list([int(1), int(2n ** 62n)])],
]);

const text = writeText(doc); // {"name":"Release 1.0","files":[1,4611686018427387904]}
const same = parseText('{ name: "Release 1.0", files: [1, 4611686018427387904], }');
const bytes = encodeBinary(doc); // the bytes the Rust reference writes
const back = decodeBinary(bytes);
```

- **Model.** `Value` is a discriminated union on `kind`: `null`, `bool`, `int`
  (a `bigint`, an `i64`), `float` (a `number` and its exact `bits`, so `-0` and
  every NaN payload survive), `str`, `list`, `map` (an ordered array of unique
  keys), `ref` (`refKind` and `id`) and `bytes` (`Uint8Array`). Build values
  with `nullValue`, `bool`, `int`, `float`, `floatFromBits`, `str`, `list`,
  `map`, `ref` and `bytes`; compare with `valuesEqual` (floats by bits).
- **Text.** `parseText` reads JSON with comments, trailing commas, unquoted
  keys and a byte order mark; a duplicate key is an error. Typed strings
  (`@kind:id`, `@bytes:...`, `@@x`) stay the strings they are written as, as in
  the reference's text parser. `parseDocument` also gives the span of every
  value. `writeText` writes compact text, floats in the shortest spelling
  that reads back as a float (`1.0`, `1e16`, `-0.0`), a ref or bytes as its
  typed string. Errors are `SyntaxError` (this module's own class) with
  `at.line` and `at.col`.
- **Binary.** `decodeBinary` reads every tag (including packed lists and
  streams, with their own string tables); `encodeBinary` writes the bytes the
  reference writes for a `Value`. Malformed input throws a `DecodeError` whose
  `kind` is the vectors' class (`bad_magic`, `bad_length`, ...); every count
  and length is checked against the bytes left before anything is allocated,
  and nesting is limited to `MAX_DEPTH` (256).
- **Frames.** `parseShapeDoc` reads a shape description (`{ root, defs }` as
  compact text) and `readFrame(bytes, shape)` reads a frame by it, including
  the backward table length, records that grew or shrank, lane-packed int lists
  and float lists. Malformed frames throw `FrameError`.
- **Streams.** `readFrames(bytes)` yields the frames of a stream
  (little-endian `u32` length, then the body) and throws `StreamError`
  (`unexpected_eof`, `invalid_data` above `MAX_FRAME`); `writeFrame(body)`
  frames one message.

## Tests

```sh
npm test            # after npm run build
```

`test/conformance.test.ts` loads `conformance/cases.json` and checks every case
in every form it has, per the vectors' README: text parse and write, binary
decode and encode, frames (and the frames read by other schemas), streams, and
every malformed input of the `text`, `text_schema`, `binary`, `frame` and `io`
forms. `test/robustness.test.ts` mutates the vectors and checks that malformed
input only ever raises this port's own error types.

## Not covered

- Packing frames (`to_frame`): frames are read, not written.
- Writing the packed binary tags (10, 13, 14) and streams: the decoder reads
  them, the encoder writes the plain forms (tags 3, 4, 6, ...), as the
  reference does for a `Value`.
- Schemas, `patch`, TOML/JSON import, `Resolve`. The one text schema the
  vectors use (`limits`) is checked by a small reader in `test/support`, not
  by the library.
- Frame shapes of an atom (a number in a process's table) are rejected.
