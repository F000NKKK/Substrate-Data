package substrate.data;

/** Compact text out of a value. */
final class TextWriter {
    private static final int FIRST_PRINTABLE = 0x20;

    private TextWriter() {}

    static void write(Value value, StringBuilder out) {
        if (value instanceof Null) {
            out.append("null");
        } else if (value instanceof Bool b) {
            out.append(b.value());
        } else if (value instanceof Int i) {
            out.append(i.value());
        } else if (value instanceof Float f) {
            out.append(FloatSpelling.of(f.value()));
        } else if (value instanceof Str s) {
            string(s.value(), out);
        } else if (value instanceof ListValue l) {
            list(l, out);
        } else if (value instanceof MapValue m) {
            map(m, out);
        } else if (value instanceof Ref r) {
            string(r.spelling(), out);
        } else if (value instanceof Bytes b) {
            string(Typed.spelling(b).orElseThrow(), out);
        }
    }

    private static void list(ListValue list, StringBuilder out) {
        out.append('[');
        boolean first = true;
        for (Value item : list.items()) {
            if (!first) {
                out.append(',');
            }
            first = false;
            write(item, out);
        }
        out.append(']');
    }

    private static void map(MapValue map, StringBuilder out) {
        out.append('{');
        boolean first = true;
        for (MapValue.Entry entry : map.entries()) {
            if (!first) {
                out.append(',');
            }
            first = false;
            string(entry.key(), out);
            out.append(':');
            write(entry.value(), out);
        }
        out.append('}');
    }

    private static void string(String text, StringBuilder out) {
        out.append('"');
        for (int i = 0; i < text.length(); i++) {
            char c = text.charAt(i);
            switch (c) {
                case '"' -> out.append("\\\"");
                case '\\' -> out.append("\\\\");
                case '\n' -> out.append("\\n");
                case '\r' -> out.append("\\r");
                case '\t' -> out.append("\\t");
                default -> {
                    if (c < FIRST_PRINTABLE) {
                        out.append(String.format("\\u%04x", (int) c));
                    } else {
                        out.append(c);
                    }
                }
            }
        }
        out.append('"');
    }
}
