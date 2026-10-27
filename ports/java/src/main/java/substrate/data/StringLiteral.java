package substrate.data;

/** Reads a double-quoted JSON string at a cursor. */
final class StringLiteral {
    private static final int HEX_DIGITS = 4;
    private static final int HEX_RADIX = 16;

    private final TextCursor cursor;
    private final StringBuilder out = new StringBuilder();

    private StringLiteral(TextCursor cursor) {
        this.cursor = cursor;
    }

    /** The string whose opening quote is at the cursor. */
    static String read(TextCursor cursor) throws TextSyntaxException {
        return new StringLiteral(cursor).run();
    }

    private String run() throws TextSyntaxException {
        int open = cursor.index();
        cursor.advance(1);
        while (true) {
            int c = cursor.peek();
            if (c < 0) {
                throw cursor.errorAt(open, "unclosed string");
            }
            if (c == '"') {
                cursor.advance(1);
                return out.toString();
            }
            if (c == '\\') {
                escape();
            } else if (c == '\n') {
                throw cursor.error("line break in a string (write `\\n`)");
            } else if (c < 0x20) {
                throw cursor.error("control character in a string");
            } else {
                out.append((char) c);
                cursor.advance(1);
            }
        }
    }

    private void escape() throws TextSyntaxException {
        int at = cursor.index();
        cursor.advance(1);
        int c = cursor.peek();
        if (c < 0) {
            throw cursor.error("unclosed string");
        }
        cursor.advance(1);
        switch (c) {
            case '"' -> out.append('"');
            case '\\' -> out.append('\\');
            case '/' -> out.append('/');
            case 'b' -> out.append('\b');
            case 'f' -> out.append('\f');
            case 'n' -> out.append('\n');
            case 'r' -> out.append('\r');
            case 't' -> out.append('\t');
            case 'u' -> unicode(at);
            default -> throw cursor.errorAt(at,
                    "unknown escape (`\\n`, `\\t`, `\\\"`, `\\\\`, `\\uXXXX`, ...)");
        }
    }

    private void unicode(int at) throws TextSyntaxException {
        int high = hex4(at);
        if (Character.isHighSurrogate((char) high)) {
            if (!cursor.startsWith("\\u")) {
                throw cursor.errorAt(at, "unpaired surrogate in `\\u` escape");
            }
            cursor.advance(2);
            int low = hex4(at);
            if (!Character.isLowSurrogate((char) low)) {
                throw cursor.errorAt(at, "unpaired surrogate in `\\u` escape");
            }
            out.append((char) high).append((char) low);
        } else if (Character.isLowSurrogate((char) high)) {
            throw cursor.errorAt(at, "unpaired surrogate in `\\u` escape");
        } else {
            out.append((char) high);
        }
    }

    private int hex4(int at) throws TextSyntaxException {
        int start = cursor.index();
        String text = cursor.text();
        if (start + HEX_DIGITS > text.length()) {
            throw cursor.errorAt(at, "`\\u` needs four hex digits");
        }
        int code = 0;
        for (int i = 0; i < HEX_DIGITS; i++) {
            int digit = Character.digit(text.charAt(start + i), HEX_RADIX);
            if (digit < 0 || text.charAt(start + i) > 'f') {
                throw cursor.errorAt(at, "`\\u` needs four hex digits");
            }
            code = code * HEX_RADIX + digit;
        }
        cursor.advance(HEX_DIGITS);
        return code;
    }
}
