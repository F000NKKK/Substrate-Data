package substrate.data;

/** Binary data or a frame that won't decode (SD0009 when read as a schema). */
public final class DecodeException extends Exception {
    private static final long serialVersionUID = 1L;

    private final DecodeErrorKind kind;
    private final int offset;

    public DecodeException(DecodeErrorKind kind, String message, int offset) {
        super("byte " + offset + ": " + message);
        this.kind = kind;
        this.offset = offset;
    }

    /** The class of the problem. */
    public DecodeErrorKind kind() {
        return kind;
    }

    /** The byte it was found at (inside a stream's body: relative to that body). */
    public int offset() {
        return offset;
    }

    /** The diagnostic code of binary data that won't decode. */
    public String code() {
        return "SD0009";
    }
}
