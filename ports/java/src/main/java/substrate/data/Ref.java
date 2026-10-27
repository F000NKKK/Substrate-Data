package substrate.data;

/**
 * A reference: a {@code kind} (lower case ASCII, {@code [a-z][a-z0-9_]*}) and
 * an opaque {@code id} (anything but empty, whitespace or control characters).
 */
public record Ref(String kind, String id) implements Value {
    /**
     * @throws IllegalArgumentException when the kind or the id is not valid
     */
    public Ref {
        if (!isValidKind(kind)) {
            throw new IllegalArgumentException("`" + kind + "` isn't a kind (lower case letters, digits and `_`)");
        }
        if (!isValidId(id)) {
            throw new IllegalArgumentException("`" + id + "` isn't an id (not empty, no spaces or control characters)");
        }
    }

    /** Whether {@code kind} may be a reference's kind. */
    public static boolean isValidKind(String kind) {
        if (kind.isEmpty() || !isLower(kind.charAt(0))) {
            return false;
        }
        return kind.chars().skip(1).allMatch(c -> isLower(c) || (c >= '0' && c <= '9') || c == '_');
    }

    /** Whether {@code id} may be a reference's id. */
    public static boolean isValidId(String id) {
        return !id.isEmpty() && id.codePoints().noneMatch(c -> isWhitespace(c) || Character.getType(c) == Character.CONTROL);
    }

    /** The spelling in text: {@code @kind:id}. */
    public String spelling() {
        return "@" + kind + ":" + id;
    }

    private static boolean isLower(int c) {
        return c >= 'a' && c <= 'z';
    }

    /** The Unicode {@code White_Space} property, as the reference reads it. */
    private static boolean isWhitespace(int c) {
        return (c >= 0x09 && c <= 0x0D)
                || c == 0x20 || c == 0x85 || c == 0xA0 || c == 0x1680
                || (c >= 0x2000 && c <= 0x200A)
                || c == 0x2028 || c == 0x2029 || c == 0x202F || c == 0x205F || c == 0x3000;
    }
}
