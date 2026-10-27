package substrate.data;

import java.util.HashSet;
import java.util.List;
import java.util.Optional;
import java.util.Set;

/**
 * A map: entries <strong>in order</strong> with unique keys. Order is part of
 * the value, so two maps with the same entries in another order are different.
 */
public record MapValue(List<Entry> entries) implements Value {
    /** One key and its value. */
    public record Entry(String key, Value value) {
        public Entry {
            java.util.Objects.requireNonNull(key, "key");
            java.util.Objects.requireNonNull(value, "value");
        }
    }

    /**
     * @throws IllegalArgumentException when a key repeats
     */
    public MapValue {
        entries = List.copyOf(entries);
        Set<String> seen = new HashSet<>();
        for (Entry entry : entries) {
            if (!seen.add(entry.key())) {
                throw new IllegalArgumentException("`" + entry.key() + "` is already set in this map");
            }
        }
    }

    /** A map of the given entries. */
    public static MapValue of(Entry... entries) {
        return new MapValue(List.of(entries));
    }

    /** How many entries. */
    public int size() {
        return entries.size();
    }

    /** The value of {@code key}, if there is one. */
    public Optional<Value> get(String key) {
        return entries.stream().filter(e -> e.key().equals(key)).map(Entry::value).findFirst();
    }
}
