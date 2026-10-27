package substrate.data;

/**
 * A value of the data model: null, bool, int, float, string, list, map,
 * reference or bytes.
 *
 * <p>A {@link Str} holds a string <em>as spelled</em>: plain text starting
 * with {@code @} is stored with it doubled, and {@code "@file:x"} stays a
 * string until a schema (or {@link Typed}) gives it a type.
 */
public sealed interface Value
        permits Null, Bool, Int, Float, Str, ListValue, MapValue, Ref, Bytes {}
