package substrate.data;

/** The null value. */
public record Null() implements Value {
    /** The only null. */
    public static final Null INSTANCE = new Null();
}
