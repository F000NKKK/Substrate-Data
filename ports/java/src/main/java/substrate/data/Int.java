package substrate.data;

/** An integer: an {@code i64} (a {@code u64} above its max is carried as its bits). */
public record Int(long value) implements Value {}
