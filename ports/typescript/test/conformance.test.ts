// Runs every case of conformance/cases.json, in every form the case has.

import assert from 'node:assert/strict';
import { after, test } from 'node:test';

import {
  DecodeError,
  SyntaxError,
  StreamError,
  decodeBinary,
  encodeBinary,
  parseShapeDoc,
  parseText,
  readFrame,
  readFrames,
  valuesEqual,
  writeText,
  type Value,
} from '../src/index.js';
import { diagnoseLimits } from './support/limits-schema.js';
import { fromHex, toHex } from './support/hex.js';
import { bytesOf, loadCases, type Case, type ErrorJson, type ModelJson } from './support/manifest.js';
import { modelValue, textModel } from './support/model-json.js';

const checked = new Map<string, number>();

function count(form: string): void {
  checked.set(form, (checked.get(form) ?? 0) + 1);
}

function expectValue(got: Value, want: Value, what: string): void {
  assert.ok(valuesEqual(got, want), `${what}: ${writeText(got)} is not ${writeText(want)}`);
}

function checkText(c: Case, text: string, value: Value): void {
  expectValue(parseText(text), textModel(value), 'parsed text');
  if (c.text_is_written === true) {
    assert.equal(writeText(value), text, 'written text');
  }
  count('text');
}

function checkBinary(c: Case, value: Value | undefined): void {
  const bytes = bytesOf(c.binary, c.binary_file);
  const decoded = decodeBinary(bytes);
  if (value !== undefined) {
    expectValue(decoded, value, 'decoded binary');
  }
  if (value === undefined || c.binary_is_encoded === true) {
    assert.equal(toHex(encodeBinary(value ?? decoded)), toHex(bytes), 'encoded binary');
  }
  count('binary');
}

function checkFrame(c: Case): void {
  const frame = c.frame;
  if (frame === undefined) {
    return;
  }
  const bytes = bytesOf(frame.bytes, frame.bytes_file);
  expectValue(readFrame(bytes, parseShapeDoc(frame.shape)), modelValue(frame.value), 'frame');
  for (const other of frame.also_read_by ?? []) {
    expectValue(readFrame(bytes, parseShapeDoc(other.shape)), modelValue(other.value), `read by ${other.schema}`);
  }
  count('frame');
}

function checkIo(c: Case): void {
  const io = c.io;
  if (io === undefined) {
    return;
  }
  const frames = [...readFrames(bytesOf(io.bytes, io.bytes_file))].map(toHex);
  assert.deepEqual(frames, io.messages);
  assert.equal(io.end, 'clean');
  count('io');
}

function expectTextError(error: ErrorJson): void {
  assert.throws(
    () => parseText(error.input),
    (e: unknown) => {
      assert.ok(e instanceof SyntaxError, 'a SyntaxError');
      assert.deepEqual({ line: e.at.line, col: e.at.col }, { line: error.line, col: error.col });
      return true;
    },
  );
}

function expectBinaryError(error: ErrorJson): void {
  assert.throws(
    () => decodeBinary(fromHex(error.input)),
    (e: unknown) => {
      assert.ok(e instanceof DecodeError, 'a DecodeError');
      assert.equal(e.kind, error.kind);
      return true;
    },
  );
}

function expectFrameError(error: ErrorJson): void {
  const shape = parseShapeDoc(error.shape ?? '');
  assert.throws(() => readFrame(fromHex(error.input), shape), { name: 'FrameError' });
}

function expectIoError(error: ErrorJson): void {
  const frames: Uint8Array[] = [];
  assert.throws(
    () => {
      for (const frame of readFrames(fromHex(error.input))) {
        frames.push(frame);
      }
    },
    (e: unknown) => {
      assert.ok(e instanceof StreamError, 'a StreamError');
      assert.equal(e.kind, error.kind);
      return true;
    },
  );
  assert.equal(frames.length, error.frames_before);
}

function expectSchemaError(error: ErrorJson): void {
  assert.deepEqual(
    diagnoseLimits(error.input).map(({ code, line, col, path }) => ({ code, line, col, path })),
    (error.diagnostics ?? []).map(({ code, line, col, path }) => ({ code, line, col, path })),
  );
}

const ERROR_CHECKS: Record<ErrorJson['form'], (error: ErrorJson) => void> = {
  text: expectTextError,
  text_schema: expectSchemaError,
  binary: expectBinaryError,
  frame: expectFrameError,
  io: expectIoError,
};

function runCase(c: Case): void {
  const value = c.value === undefined ? undefined : modelValue(c.value as ModelJson);
  if (c.text !== undefined && value !== undefined) {
    checkText(c, c.text, value);
  }
  if (c.binary !== undefined || c.binary_file !== undefined) {
    checkBinary(c, value);
  }
  checkFrame(c);
  checkIo(c);
  if (c.error !== undefined) {
    ERROR_CHECKS[c.error.form](c.error);
    count(`error:${c.error.form}`);
  }
}

const cases = loadCases();

test('the manifest has the 245 cases of this version', () => {
  assert.ok(cases.length >= 245, `${cases.length} cases`);
  assert.equal(new Set(cases.map((c) => c.name)).size, cases.length, 'unique names');
});

for (const c of cases) {
  test(c.name, () => runCase(c));
}

after(() => {
  const rows = [...checked].sort().map(([form, n]) => `${form}: ${n}`);
  console.log(`conformance checks per form: ${rows.join(', ')}`);
});
