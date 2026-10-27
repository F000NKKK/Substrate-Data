// Numbers bit-packed in lanes: what a frame's list of ints is made of.
//
// A list of `n` numbers of `width` bits (1 to 32) is `n / 256` groups, then
// the rest packed one after another, little-endian bits. A group is 256
// numbers in 8 lanes of 32-bit words: number `j` goes to lane `j % 8`, the
// `j / 8`-th in it; word `k` of lane `l` is the group's `k * 8 + l`-th word.
// Numbers wider than 32 bits go one after another with no groups.

export const LANES = 8;
export const GROUP = 256;
export const GROUP_MAX_BITS = 32;
const WORD_BYTES = 4;
const BYTE_BITS = 8;

function groupBytes(width: number): number {
  return LANES * width * WORD_BYTES;
}

function split(n: number, width: number): { groups: number; rest: number } {
  if (width > GROUP_MAX_BITS) {
    return { groups: 0, rest: n };
  }
  return { groups: Math.floor(n / GROUP), rest: n % GROUP };
}

/** The bytes `n` numbers of `width` bits take. */
export function packedBytes(n: number, width: number): number {
  const { groups, rest } = split(n, width);
  return groups * groupBytes(width) + Math.ceil((rest * width) / BYTE_BITS);
}

/** Unpacks `n` numbers of `width` bits from `raw` (which holds exactly `packedBytes(n, width)` bytes). */
export function unpackLanes(raw: Uint8Array, n: number, width: number): bigint[] {
  const view = new DataView(raw.buffer, raw.byteOffset, raw.length);
  const { groups, rest } = split(n, width);
  const out: bigint[] = [];
  for (let g = 0; g < groups; g += 1) {
    for (let j = 0; j < GROUP; j += 1) {
      out.push(BigInt(groupedNumber(view, g * groupBytes(width), j, width)));
    }
  }
  const tail = groups * groupBytes(width);
  for (let i = 0; i < rest; i += 1) {
    out.push(sequentialNumber(raw, tail, i, width));
  }
  return out;
}

/** Number `j` of the group at byte `start`: lane `j % 8`, its `j / 8`-th number. */
function groupedNumber(view: DataView, start: number, j: number, width: number): number {
  const lane = j % LANES;
  const bit = Math.floor(j / LANES) * width;
  const word = Math.floor(bit / GROUP_MAX_BITS);
  const shift = bit % GROUP_MAX_BITS;
  const at = (k: number): number => view.getUint32(start + (k * LANES + lane) * WORD_BYTES, true);
  let value = at(word) >>> shift;
  if (shift + width > GROUP_MAX_BITS) {
    value |= at(word + 1) << (GROUP_MAX_BITS - shift);
  }
  return width === GROUP_MAX_BITS ? value >>> 0 : (value & (2 ** width - 1)) >>> 0;
}

/** Number `i` of numbers packed one after another from byte `start`. */
function sequentialNumber(raw: Uint8Array, start: number, i: number, width: number): bigint {
  const bit = i * width;
  const first = start + Math.floor(bit / BYTE_BITS);
  const shift = bit % BYTE_BITS;
  const span = Math.ceil((shift + width) / BYTE_BITS);
  let acc = 0n;
  for (let k = 0; k < span; k += 1) {
    acc |= BigInt(raw[first + k] ?? 0) << BigInt(k * BYTE_BITS);
  }
  return (acc >> BigInt(shift)) & ((1n << BigInt(width)) - 1n);
}
