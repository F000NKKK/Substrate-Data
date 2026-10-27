/** Which kind of malformed input: what a port must agree on with the reference. */
export type DecodeErrorKind =
  | 'bad_magic'
  | 'bad_version'
  | 'unexpected_end'
  | 'unknown_tag'
  | 'bad_varint'
  | 'bad_length'
  | 'bad_utf8'
  | 'bad_string_index'
  | 'bad_reference'
  | 'duplicate_key'
  | 'trailing_bytes'
  | 'too_deep';

/** Malformed binary data (code SD0009). */
export class DecodeError extends Error {
  constructor(
    readonly kind: DecodeErrorKind,
    message: string,
    /** The byte it was found at (inside a stream's body, relative to that body). */
    readonly offset: number,
  ) {
    super(`byte ${offset}: ${message}`);
    this.name = 'DecodeError';
  }
}
