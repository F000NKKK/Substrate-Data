import type { Cursor } from './cursor.js';

const HEX4 = /^[0-9a-fA-F]{4}$/;
const SIMPLE_ESCAPES: Readonly<Record<string, string>> = {
  '"': '"',
  '\\': '\\',
  '/': '/',
  b: '\b',
  f: '\f',
  n: '\n',
  r: '\r',
  t: '\t',
};

/** Reads a string literal; the cursor is on its opening quote. */
export function readString(c: Cursor): string {
  const open = c.pos;
  c.pos += 1;
  let out = '';
  for (;;) {
    const stop = findStop(c.text, c.pos);
    if (stop === -1) {
      throw c.errorAt(open, 'unclosed string');
    }
    out += c.text.slice(c.pos, stop);
    c.pos = stop;
    const ch = c.text[stop];
    if (ch === '"') {
      c.pos += 1;
      return out;
    }
    if (ch === '\\') {
      out += readEscape(c);
    } else if (ch === '\n') {
      throw c.error('line break in a string (write `\\n`)');
    } else {
      throw c.error('control character in a string');
    }
  }
}

/** The first quote, backslash or control character at or after `from`. */
function findStop(text: string, from: number): number {
  for (let i = from; i < text.length; i += 1) {
    const unit = text.charCodeAt(i);
    if (unit === 0x22 || unit === 0x5c || unit < 0x20) {
      return i;
    }
  }
  return -1;
}

function readEscape(c: Cursor): string {
  const at = c.pos;
  c.pos += 1;
  const ch = c.peek();
  if (ch === undefined) {
    throw c.error('unclosed string');
  }
  c.pos += 1;
  if (ch === 'u') {
    return readUnicode(c, at);
  }
  const simple = SIMPLE_ESCAPES[ch];
  if (simple === undefined) {
    throw c.errorAt(at, 'unknown escape (`\\n`, `\\t`, `\\"`, `\\\\`, `\\uXXXX`, ...)');
  }
  return simple;
}

function readUnicode(c: Cursor, at: number): string {
  const high = readHex4(c, at);
  if (high >= 0xdc00 && high < 0xe000) {
    throw c.errorAt(at, 'unpaired surrogate in `\\u` escape');
  }
  if (high < 0xd800 || high >= 0xdc00) {
    return String.fromCodePoint(high);
  }
  if (!c.startsWith('\\u')) {
    throw c.errorAt(at, 'unpaired surrogate in `\\u` escape');
  }
  c.pos += 2;
  const low = readHex4(c, at);
  if (low < 0xdc00 || low >= 0xe000) {
    throw c.errorAt(at, 'unpaired surrogate in `\\u` escape');
  }
  return String.fromCodePoint(0x10000 + ((high - 0xd800) << 10) + (low - 0xdc00));
}

function readHex4(c: Cursor, at: number): number {
  const digits = c.text.slice(c.pos, c.pos + 4);
  if (!HEX4.test(digits)) {
    throw c.errorAt(at, '`\\u` needs four hex digits');
  }
  c.pos += 4;
  return Number.parseInt(digits, 16);
}
