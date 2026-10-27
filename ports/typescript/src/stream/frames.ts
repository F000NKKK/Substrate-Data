// Messages on a byte stream: each a little-endian u32 length, then that many bytes.

/** The longest frame read; anything longer is a broken or hostile peer. */
export const MAX_FRAME = 256 << 20;
const LENGTH_BYTES = 4;

export type StreamErrorKind = 'unexpected_eof' | 'invalid_data' | 'invalid_input';

/** A stream that ends inside a frame, a frame that is too long. */
export class StreamError extends Error {
  constructor(
    readonly kind: StreamErrorKind,
    message: string,
  ) {
    super(message);
    this.name = 'StreamError';
  }
}

/**
 * The frames of `bytes` in order. The stream ends cleanly only between
 * frames; one cut inside a length or a body, or a length above
 * `MAX_FRAME`, is an error thrown when the iteration reaches it.
 */
export function* readFrames(bytes: Uint8Array): Generator<Uint8Array, void, undefined> {
  let pos = 0;
  while (pos < bytes.length) {
    if (bytes.length - pos < LENGTH_BYTES) {
      throw new StreamError('unexpected_eof', 'the stream ends inside a length');
    }
    const len = new DataView(bytes.buffer, bytes.byteOffset + pos, LENGTH_BYTES).getUint32(0, true);
    if (len > MAX_FRAME) {
      throw new StreamError('invalid_data', 'frame too long');
    }
    pos += LENGTH_BYTES;
    if (len > bytes.length - pos) {
      throw new StreamError('unexpected_eof', 'the stream ends inside a frame');
    }
    yield bytes.subarray(pos, pos + len);
    pos += len;
  }
}

/** `body` (a message's binary form) as one frame: its length, then it. */
export function writeFrame(body: Uint8Array): Uint8Array {
  if (body.length > MAX_FRAME) {
    throw new StreamError('invalid_input', 'frame too long');
  }
  const out = new Uint8Array(LENGTH_BYTES + body.length);
  new DataView(out.buffer).setUint32(0, body.length, true);
  out.set(body, LENGTH_BYTES);
  return out;
}
