/** The four bytes every document starts with. */
export const MAGIC = new Uint8Array([0x53, 0x44, 0x41, 0x54]);
/** The only version this reads and writes. */
export const VERSION = 1;
/** Nesting deeper than this is malformed. */
export const MAX_DEPTH = 256;
/** The low bit of a string marker that says the string's bytes follow. */
export const NEW_STRING = 1n;
/** A stream's byte length is this many bytes, little-endian. */
export const STREAM_LEN_BYTES = 4;

export const Tag = {
  Null: 0,
  False: 1,
  True: 2,
  Int: 3,
  Float: 4,
  Str: 5,
  List: 6,
  Map: 7,
  Ref: 8,
  Bytes: 9,
  Ints: 10,
  Stream: 11,
  Float32: 12,
  Floats: 13,
  Floats32: 14,
} as const;
