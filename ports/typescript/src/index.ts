// Substrate Data: a TypeScript port, checked against the conformance vectors.

export {
  bool,
  bytes,
  float,
  floatFromBits,
  int,
  list,
  map,
  nullValue,
  ref,
  str,
  I64_MAX,
  I64_MIN,
} from './model/value.js';
export type {
  BoolValue,
  BytesValue,
  FloatValue,
  IntValue,
  ListValue,
  MapEntry,
  MapValue,
  NullValue,
  RefValue,
  StrValue,
  Value,
} from './model/value.js';
export { valuesEqual } from './model/equal.js';
export { InvalidValueError } from './model/errors.js';
export { isKindName, isRefId } from './model/reference.js';

export { parseDocument, parseText } from './text/parser.js';
export type { Document } from './text/parser.js';
export { writeText, typedSpelling } from './text/writer.js';
export { SyntaxError } from './text/syntax-error.js';
export { lineColOf } from './text/line-col.js';
export type { LineCol } from './text/line-col.js';
export type { Span } from './text/spans.js';

export { decodeBinary } from './binary/decoder.js';
export { encodeBinary } from './binary/encoder.js';
export { DecodeError } from './binary/error.js';
export type { DecodeErrorKind } from './binary/error.js';
export { MAX_DEPTH, VERSION } from './binary/layout.js';

export { readFrame } from './frame/read-shaped.js';
export { parseShapeDoc } from './frame/shape-doc.js';
export { FrameError } from './frame/error.js';
export type { Shape, ShapeDoc, Field } from './frame/shape.js';

export { MAX_FRAME, StreamError, readFrames, writeFrame } from './stream/frames.js';
export type { StreamErrorKind } from './stream/frames.js';
