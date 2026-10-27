// Float bits: a JavaScript number cannot carry every NaN payload, so a float
// value keeps its IEEE 754 bits next to the number.

const scratch = new DataView(new ArrayBuffer(8));

const F32_EXPONENT_ALL = 0xff;
const F32_MANTISSA_BITS = 23;
const F32_MANTISSA_MASK = 0x7f_ffff;
const F64_QUIET_BIT = 1n << 51n;

/** The IEEE 754 binary64 bits of `n` (every NaN becomes the canonical one). */
export function bitsOf(n: number): bigint {
  scratch.setFloat64(0, n);
  return scratch.getBigUint64(0);
}

/** The number with the binary64 bits `bits`. */
export function numberOf(bits: bigint): number {
  scratch.setBigUint64(0, BigInt.asUintN(64, bits));
  return scratch.getFloat64(0);
}

/**
 * The binary64 bits of the binary32 with bits `word`, widened the way a
 * processor does: exact, and a NaN keeps its payload and becomes quiet.
 */
export function widenBits(word: number): bigint {
  const sign = BigInt(word >>> 31) << 63n;
  const exponent = (word >>> F32_MANTISSA_BITS) & F32_EXPONENT_ALL;
  const mantissa = word & F32_MANTISSA_MASK;
  if (exponent === F32_EXPONENT_ALL) {
    const payload = BigInt(mantissa) << 29n;
    const quiet = mantissa === 0 ? 0n : F64_QUIET_BIT;
    return sign | (0x7ffn << 52n) | payload | quiet;
  }
  scratch.setUint32(0, word);
  return bitsOf(scratch.getFloat32(0));
}

/** The 32 bits of `n` as a binary32 (rounded), for tests and writers. */
export function narrowWord(n: number): number {
  scratch.setFloat32(0, n);
  return scratch.getUint32(0);
}
