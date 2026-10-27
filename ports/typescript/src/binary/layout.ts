/** The four bytes every document starts with. */
export const MAGIC = new Uint8Array([0x53, 0x44, 0x41, 0x54]);
/** The version this writes. */
export const VERSION = 2;
/** The older version this still reads (never written). */
export const LEGACY_VERSION = 1;
/** The first of the 128 tags that are an int by themselves (version 2). */
export const INT_INLINE = 128;
/** The smallest int a tag of its own is: `INT_INLINE` is this. */
export const INLINE_INT_MIN = -16;
/** The largest int a tag of its own is. */
export const INLINE_INT_MAX = 111;
/** The first of the 32 tags that are a table string by themselves (version 2). */
export const STR_INDEX = 32;
/** How many strings a tag of its own names. */
export const STR_INDEX_COUNT = 32;
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
  MapShape: 15,
  Records: 16,
} as const;
