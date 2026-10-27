package substrate.data;

/** A string written as a typed value ({@code @...}) that isn't one. */
public final class TypedException extends Exception {
    private static final long serialVersionUID = 1L;

    public TypedException(String message) {
        super(message);
    }
}
