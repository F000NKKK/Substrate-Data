package substrate.data;

/**
 * The binary form, SDAT version 2 (version 1 is still read, never written): {@code "SDAT" version:u8 value}. Strings are
 * interned on the way, nothing is aligned, and decoding checks every count and
 * length against the bytes left and nesting against {@link #MAX_DEPTH}.
 */
public final class Binary {
    /** What a binary document starts with. */
    static final byte[] MAGIC = {'S', 'D', 'A', 'T'};
    /** The version byte after the magic: what is written. */
    public static final int VERSION = 2;
    /** The older version that is still read. */
    public static final int LEGACY_VERSION = 1;
    /** Nested deeper than this is malformed. */
    public static final int MAX_DEPTH = 256;
    /** A stream's length: fixed width, little-endian. */
    public static final int STREAM_LEN_BYTES = 4;
    /** The low bit of a string's varint: set for a new string, clear for a table index. */
    public static final long NEW_STRING = 1;

    private Binary() {}

    /**
     * Reads a whole document. A packed list or a stream reads as a {@link ListValue}.
     *
     * @throws DecodeException when the bytes are not the binary form
     */
    public static Value decode(byte[] bytes) throws DecodeException {
        return BinaryDecoder.document(bytes);
    }

    /**
     * Reads a value held without a header, with a string table of its own (a
     * tagged enum in a frame).
     *
     * @throws DecodeException when the bytes are not a value
     */
    public static Value decodeBody(byte[] bytes) throws DecodeException {
        return BinaryDecoder.body(bytes);
    }

    /** {@code value} as a whole document, version 2: reading a version 1 document and writing it upgrades it. */
    public static byte[] encode(Value value) {
        return BinaryEncoder.document(value);
    }
}
