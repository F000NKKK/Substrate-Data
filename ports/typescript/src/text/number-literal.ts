import { float, int, I64_MAX, I64_MIN, type Value } from '../model/value.js';
import type { Cursor } from './cursor.js';

function isDigit(c: string | undefined): boolean {
  return c !== undefined && c >= '0' && c <= '9';
}

/** Skips digits; how many there were. */
function skipDigits(c: Cursor): number {
  const start = c.pos;
  while (isDigit(c.peek())) {
    c.pos += 1;
  }
  return c.pos - start;
}

/**
 * Reads a number: an int when it has no fraction or exponent and fits an
 * `i64`, else a float (so an integer past `i64` reads as a float).
 */
export function readNumber(c: Cursor): Value {
  const start = c.pos;
  if (c.peek() === '-') {
    c.pos += 1;
  }
  const intStart = c.pos;
  if (skipDigits(c) === 0) {
    throw c.error('expected digits');
  }
  if (c.text[intStart] === '0' && c.pos - intStart > 1) {
    throw c.errorAt(intStart, "numbers don't start with 0");
  }
  const hasFraction = skipFraction(c);
  const hasExponent = skipExponent(c);
  const isFloat = hasFraction || hasExponent;
  const spelled = c.text.slice(start, c.pos);
  if (!isFloat) {
    const n = BigInt(spelled);
    if (n >= I64_MIN && n <= I64_MAX) {
      return int(n);
    }
  }
  return float(Number(spelled));
}

function skipFraction(c: Cursor): boolean {
  if (c.peek() !== '.') {
    return false;
  }
  c.pos += 1;
  if (skipDigits(c) === 0) {
    throw c.error('expected digits after `.`');
  }
  return true;
}

function skipExponent(c: Cursor): boolean {
  const e = c.peek();
  if (e !== 'e' && e !== 'E') {
    return false;
  }
  c.pos += 1;
  const sign = c.peek();
  if (sign === '+' || sign === '-') {
    c.pos += 1;
  }
  if (skipDigits(c) === 0) {
    throw c.error('expected digits in the exponent');
  }
  return true;
}
