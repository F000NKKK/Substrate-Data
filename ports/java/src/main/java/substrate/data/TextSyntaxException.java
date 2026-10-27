package substrate.data;

/** Text that doesn't parse: SD0001, at a 1-based line and column (in characters). */
public final class TextSyntaxException extends Exception {
    private static final long serialVersionUID = 1L;

    private final int line;
    private final int column;

    public TextSyntaxException(String message, int line, int column) {
        super(message);
        this.line = line;
        this.column = column;
    }

    /** The line, from 1. */
    public int line() {
        return line;
    }

    /** The column in characters, from 1. */
    public int column() {
        return column;
    }

    /** The diagnostic code of text that doesn't parse. */
    public String code() {
        return "SD0001";
    }
}
