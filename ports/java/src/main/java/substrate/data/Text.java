package substrate.data;

/** The text form: JSON plus comments, trailing commas and unquoted keys. */
public final class Text {
    private Text() {}

    /**
     * Parses text. Typed strings ({@code "@file:x"}, {@code "@bytes:..."}) stay
     * {@link Str}s as written; {@link Typed#classify} gives them their type.
     *
     * @throws TextSyntaxException when the text doesn't parse (SD0001)
     */
    public static Value parse(String text) throws TextSyntaxException {
        return TextParser.parse(text);
    }

    /**
     * {@code value} as compact text, typed values in their string forms, a
     * float in the shortest spelling that reads back as the same float.
     */
    public static String write(Value value) {
        StringBuilder out = new StringBuilder();
        TextWriter.write(value, out);
        return out.toString();
    }
}
