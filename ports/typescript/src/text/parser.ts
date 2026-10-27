import { bool, nullValue, str, type MapEntry, type Value } from '../model/value.js';
import { Cursor, MAX_DEPTH } from './cursor.js';
import { readNumber } from './number-literal.js';
import type { Span } from './spans.js';
import { readString } from './string-literal.js';

/** A parsed text: its value and where every part of it is. */
export interface Document {
  readonly value: Value;
  readonly spans: Span;
}

interface Parsed {
  readonly value: Value;
  readonly span: Span;
}

/** Parses text (JSON with comments, trailing commas and unquoted keys). */
export function parseDocument(text: string): Document {
  const c = new Cursor(text);
  if (text.startsWith('﻿')) {
    c.pos = 1;
  }
  c.skipTrivia();
  const { value, span } = readValue(c);
  c.skipTrivia();
  if (c.pos < text.length) {
    throw c.error('unexpected text after the value');
  }
  return { value, spans: span };
}

/**
 * Parses text to a value. Typed strings (`@kind:id`, `@bytes:...`, `@@x`)
 * stay the strings they are written as; a schema gives them their type.
 *
 * @throws SyntaxError when the text does not parse.
 */
export function parseText(text: string): Value {
  return parseDocument(text).value;
}

function readValue(c: Cursor): Parsed {
  const start = c.pos;
  const { value, children } = readAny(c);
  return { value, span: { start, end: c.pos, children } };
}

function readAny(c: Cursor): { value: Value; children: readonly Span[] } {
  const ch = c.peek();
  if (ch === '{') {
    return nested(c, readMap);
  }
  if (ch === '[') {
    return nested(c, readList);
  }
  return { value: readScalar(c, ch), children: [] };
}

function readScalar(c: Cursor, ch: string | undefined): Value {
  if (ch === '"') {
    return str(readString(c));
  }
  if (ch === '-' || (ch !== undefined && ch >= '0' && ch <= '9')) {
    return readNumber(c);
  }
  if (ch === 't' && c.startsWith('true')) {
    c.pos += 4;
    return bool(true);
  }
  if (ch === 'f' && c.startsWith('false')) {
    c.pos += 5;
    return bool(false);
  }
  if (ch === 'n' && c.startsWith('null')) {
    c.pos += 4;
    return nullValue;
  }
  throw c.error(ch === "'" ? 'strings use double quotes' : 'expected a value');
}

function nested(
  c: Cursor,
  read: (c: Cursor) => { value: Value; children: readonly Span[] },
): { value: Value; children: readonly Span[] } {
  c.depth += 1;
  if (c.depth > MAX_DEPTH) {
    throw c.error(`nested deeper than ${MAX_DEPTH} levels`);
  }
  const result = read(c);
  c.depth -= 1;
  return result;
}

function readList(c: Cursor): { value: Value; children: readonly Span[] } {
  c.pos += 1;
  const items: Value[] = [];
  const children: Span[] = [];
  for (;;) {
    c.skipTrivia();
    if (c.peek() === ']') {
      c.pos += 1;
      break;
    }
    const item = readValue(c);
    items.push(item.value);
    children.push(item.span);
    c.skipTrivia();
    const next = c.peek();
    if (next === ',') {
      c.pos += 1;
    } else if (next === ']') {
      c.pos += 1;
      break;
    } else {
      throw c.error('expected `,` or `]`');
    }
  }
  return { value: { kind: 'list', items }, children };
}

function readMap(c: Cursor): { value: Value; children: readonly Span[] } {
  c.pos += 1;
  const entries: MapEntry[] = [];
  const children: Span[] = [];
  const seen = new Set<string>();
  for (;;) {
    c.skipTrivia();
    if (c.peek() === '}') {
      c.pos += 1;
      break;
    }
    const keyStart = c.pos;
    const key = readKey(c);
    if (seen.has(key)) {
      throw c.errorAt(keyStart, `\`${key}\` is already set in this object`);
    }
    seen.add(key);
    const entry = readEntryValue(c);
    entries.push([key, entry.value]);
    children.push({ ...entry.span, keyStart });
    c.skipTrivia();
    const next = c.peek();
    if (next === ',') {
      c.pos += 1;
    } else if (next === '}') {
      c.pos += 1;
      break;
    } else {
      throw c.error('expected `,` or `}`');
    }
  }
  return { value: { kind: 'map', entries }, children };
}

function readEntryValue(c: Cursor): Parsed {
  c.skipTrivia();
  if (c.peek() !== ':') {
    throw c.error('expected `:` after the key');
  }
  c.pos += 1;
  c.skipTrivia();
  return readValue(c);
}

const KEY_START = /^[A-Za-z_$]$/;
const KEY_PART = /^[A-Za-z0-9_$]$/;

/** An object's key: a string, or a bare name. */
function readKey(c: Cursor): string {
  const first = c.peek();
  if (first === '"') {
    return readString(c);
  }
  if (first === undefined || !KEY_START.test(first)) {
    throw c.error('expected a key');
  }
  const start = c.pos;
  while (KEY_PART.test(c.peek() ?? '')) {
    c.pos += 1;
  }
  return c.text.slice(start, c.pos);
}

