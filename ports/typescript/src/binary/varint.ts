// Varints (LEB128 of a u64) and zigzag.

import type { ByteSink } from './byte-buffer.js';

const LOW_SEVEN = 0x7f;
const MORE = 0x80;
const LAST_SHIFT = 63;
const MAX_SHIFT = 64;

/** A place in a byte array that reads move forward. */
export interface Position {
  pos: number;
}

/**
 * The varint at `at.pos` in `bytes`, `at.pos` moved past it; `undefined`
 * when it is cut short, longer than ten bytes or past 64 bits.
 */
export function readVarint(bytes: Uint8Array, at: Position): bigint | undefined {
  let n = 0n;
  for (let shift = 0; shift < MAX_SHIFT; shift += 7) {
    const byte = bytes[at.pos];
    if (byte === undefined) {
      return undefined;
    }
    at.pos += 1;
    const bits = byte & LOW_SEVEN;
    if (shift === LAST_SHIFT && bits > 1) {
      return undefined;
    }
    n |= BigInt(bits) << BigInt(shift);
    if ((byte & MORE) === 0) {
      return n;
    }
  }
  return undefined;
}

/** `n` as a varint onto `out`. */
export function writeVarint(out: ByteSink, n: bigint | number): void {
  let rest = BigInt(n);
  while (rest > 0x7fn) {
    out.push(Number(rest & 0x7fn) | MORE);
    rest >>= 7n;
  }
  out.push(Number(rest));
}

/** `n` zigzagged: small negative numbers stay small varints. */
export function zigzag(n: bigint): bigint {
  return BigInt.asUintN(64, (n << 1n) ^ (n >> 63n));
}

/** A zigzagged `z` back. */
export function unzigzag(z: bigint): bigint {
  return (z >> 1n) ^ -(z & 1n);
}
