package substrate.data;

/** The class of malformed data; wording and offsets may differ from the reference, the class may not. */
public enum DecodeErrorKind {
    /** No {@code SDAT} header. */
    BAD_MAGIC,
    /** A version byte other than 1. */
    BAD_VERSION,
    /** Input ends where a byte is needed. */
    UNEXPECTED_END,
    /** A tag byte not in 0..=14. */
    UNKNOWN_TAG,
    /** A varint cut short, longer than 10 bytes, or with a 10th byte above 1. */
    BAD_VARINT,
    /** A count or length past the bytes left (checked before any allocation). */
    BAD_LENGTH,
    /** A string that is not UTF-8. */
    BAD_UTF8,
    /** A string index not yet in the table. */
    BAD_STRING_INDEX,
    /** A reference whose kind or id is not valid. */
    BAD_REFERENCE,
    /** A map key that repeats. */
    DUPLICATE_KEY,
    /** Bytes after the root value (or after a stream's items). */
    TRAILING_BYTES,
    /** Nesting beyond {@link Binary#MAX_DEPTH}. */
    TOO_DEEP,
    /** A frame that does not fit its shape. */
    BAD_FRAME
}
