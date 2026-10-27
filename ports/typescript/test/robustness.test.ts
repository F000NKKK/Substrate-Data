// Malformed input is an error of this port's own types, never another exception.

import assert from 'node:assert/strict';
import { test } from 'node:test';

import {
  DecodeError,
  FrameError,
  StreamError,
  SyntaxError,
  decodeBinary,
  parseShapeDoc,
  parseText,
  readFrame,
  readFrames,
} from '../src/index.js';
import { bytesOf, loadCases } from './support/manifest.js';

/** A small deterministic generator. */
function lcg(seed: number): () => number {
  let state = seed;
  return () => {
    state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
    return state;
  };
}

function mutate(bytes: Uint8Array, next: () => number): Uint8Array {
  const out = bytes.slice();
  const flips = 1 + (next() % 3);
  for (let i = 0; i < flips && out.length > 0; i += 1) {
    out[next() % out.length] = next() & 0xff;
  }
  return next() % 4 === 0 ? out.subarray(0, next() % (out.length + 1)) : out;
}

function expectOnly<E extends Error>(kind: new (...args: never[]) => E, run: () => unknown): void {
  try {
    run();
  } catch (e) {
    assert.ok(e instanceof kind, `an unexpected exception: ${String(e)}`);
  }
}

test('mutated binary documents decode or fail with a DecodeError', () => {
  const next = lcg(1);
  for (const c of loadCases()) {
    if (c.binary === undefined && c.binary_file === undefined) {
      continue;
    }
    const bytes = bytesOf(c.binary, c.binary_file);
    for (let round = 0; round < 30; round += 1) {
      expectOnly(DecodeError, () => decodeBinary(mutate(bytes, next)));
    }
  }
});

test('mutated frames read or fail with a FrameError', () => {
  const next = lcg(2);
  for (const c of loadCases()) {
    if (c.frame === undefined) {
      continue;
    }
    const bytes = bytesOf(c.frame.bytes, c.frame.bytes_file);
    const shape = parseShapeDoc(c.frame.shape);
    for (let round = 0; round < 30; round += 1) {
      expectOnly(FrameError, () => readFrame(mutate(bytes, next), shape));
    }
  }
});

test('mutated streams read or fail with a StreamError', () => {
  const next = lcg(3);
  for (const c of loadCases()) {
    if (c.io === undefined) {
      continue;
    }
    const bytes = bytesOf(c.io.bytes, c.io.bytes_file);
    for (let round = 0; round < 30; round += 1) {
      expectOnly(StreamError, () => [...readFrames(mutate(bytes, next))]);
    }
  }
});

test('mutated text parses or fails with a SyntaxError', () => {
  const next = lcg(4);
  for (const c of loadCases()) {
    const text = c.text ?? c.error?.input;
    if (text === undefined || c.error?.form === 'binary') {
      continue;
    }
    for (let round = 0; round < 30; round += 1) {
      const at = next() % (text.length + 1);
      const mutated = text.slice(0, at) + String.fromCharCode(next() % 128) + text.slice(at + 1);
      expectOnly(SyntaxError, () => parseText(mutated));
    }
  }
});

test('a lying length never allocates by it', () => {
  const huge = Uint8Array.from([0x53, 0x44, 0x41, 0x54, 1, 13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f]);
  assert.throws(() => decodeBinary(huge), { kind: 'bad_length' });
});
