package substrate.data;

import java.util.Base64;
import java.util.Optional;

/**
 * The typed values' string forms, one grammar for every format:
 * {@code "@kind:id"} is a {@link Ref}, {@code "@bytes:<base64>"} is
 * {@link Bytes}, {@code "@@text"} is the plain string {@code @text}, and any
 * other string starting with {@code @} is an error.
 */
public final class Typed {
    private static final String MARK = "@";
    private static final String BYTES_KIND = "bytes";

    private Typed() {}

    /**
     * What {@code spelled} spells: a {@link Ref}, {@link Bytes} or a plain
     * {@link Str} (its {@code @@} escape undone).
     *
     * @throws TypedException when it starts with {@code @} but is not a typed value
     */
    public static Value classify(String spelled) throws TypedException {
        if (!spelled.startsWith(MARK)) {
            return new Str(spelled);
        }
        String rest = spelled.substring(1);
        if (rest.startsWith(MARK)) {
            return new Str(rest);
        }
        int colon = rest.indexOf(':');
        if (colon < 0) {
            throw new TypedException("`" + spelled + "`: a reference is `@kind:id`; write `@@` for a literal `@`");
        }
        String kind = rest.substring(0, colon);
        String id = rest.substring(colon + 1);
        if (kind.equals(BYTES_KIND)) {
            return new Bytes(decodeBase64(id));
        }
        return reference(kind, id);
    }

    /** The string form of a typed value; empty for the untyped ones. */
    public static Optional<String> spelling(Value value) {
        if (value instanceof Ref r) {
            return Optional.of(r.spelling());
        } else if (value instanceof Bytes b) {
            return Optional.of(MARK + BYTES_KIND + ":" + Base64.getEncoder().encodeToString(b.data()));
        }
        return Optional.empty();
    }

    /** {@code value} with its typed values as their strings (plain values only). */
    public static Value flatten(Value value) {
        if (value instanceof ListValue l) {
            return new ListValue(l.items().stream().map(Typed::flatten).toList());
        } else if (value instanceof MapValue m) {
            return new MapValue(m.entries().stream()
                    .map(e -> new MapValue.Entry(e.key(), flatten(e.value()))).toList());
        }
        return spelling(value).<Value>map(Str::new).orElse(value);
    }

    /** {@code text} with a leading {@code @} doubled, so it reads back as plain text. */
    public static String escapeAt(String text) {
        return text.startsWith(MARK) ? MARK + text : text;
    }

    private static Ref reference(String kind, String id) throws TypedException {
        if (!Ref.isValidKind(kind)) {
            throw new TypedException("`" + kind + "` isn't a kind (lower case letters, digits and `_`)");
        }
        if (!Ref.isValidId(id)) {
            throw new TypedException("`@" + kind + ":" + id + "`: not an id (not empty, no spaces or control characters)");
        }
        return new Ref(kind, id);
    }

    private static byte[] decodeBase64(String text) throws TypedException {
        try {
            byte[] decoded = Base64.getDecoder().decode(text);
            if (!Base64.getEncoder().encodeToString(decoded).equals(text)) {
                throw new TypedException("`@bytes:`: not canonical base64");
            }
            return decoded;
        } catch (IllegalArgumentException e) {
            throw new TypedException("`@bytes:`: not base64 (" + e.getMessage() + ")");
        }
    }
}
