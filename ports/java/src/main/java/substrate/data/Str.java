package substrate.data;

import java.util.Objects;

/** A string as spelled (see {@link Value}). */
public record Str(String value) implements Value {
    public Str {
        Objects.requireNonNull(value, "value");
    }
}
