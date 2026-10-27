package substrate.data;

import java.util.List;

/** A list of values. */
public record ListValue(List<Value> items) implements Value {
    public ListValue {
        items = List.copyOf(items);
    }

    /** A list of the given values. */
    public static ListValue of(Value... items) {
        return new ListValue(List.of(items));
    }
}
