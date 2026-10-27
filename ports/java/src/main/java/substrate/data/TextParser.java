package substrate.data;

import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Set;

/** Text to a {@link Value}: JSON with comments, trailing commas and unquoted keys. */
final class TextParser {
    /** Deeper than this is an error, not a stack overflow. */
    static final int MAX_DEPTH = 256;

    private final TextCursor cursor;
    private int depth;

    private TextParser(String text) {
        this.cursor = new TextCursor(text);
    }

    static Value parse(String text) throws TextSyntaxException {
        TextParser parser = new TextParser(text);
        parser.cursor.skipBlank();
        Value value = parser.value();
        parser.cursor.skipBlank();
        if (!parser.cursor.atEnd()) {
            throw parser.cursor.error("unexpected text after the value");
        }
        return value;
    }

    private Value value() throws TextSyntaxException {
        int c = cursor.peek();
        if (c == '{') {
            return nested(this::map);
        }
        if (c == '[') {
            return nested(this::list);
        }
        if (c == '"') {
            return new Str(StringLiteral.read(cursor));
        }
        if (c == '-' || (c >= '0' && c <= '9')) {
            return NumberLiteral.read(cursor);
        }
        return keyword(c);
    }

    private Value keyword(int first) throws TextSyntaxException {
        if (first == 't' && cursor.startsWith("true")) {
            cursor.advance(4);
            return new Bool(true);
        }
        if (first == 'f' && cursor.startsWith("false")) {
            cursor.advance(5);
            return new Bool(false);
        }
        if (first == 'n' && cursor.startsWith("null")) {
            cursor.advance(4);
            return Null.INSTANCE;
        }
        if (first == '\'') {
            throw cursor.error("strings use double quotes");
        }
        throw cursor.error("expected a value");
    }

    /** A step that reads a container. */
    private interface Step {
        Value read() throws TextSyntaxException;
    }

    private Value nested(Step step) throws TextSyntaxException {
        depth++;
        if (depth > MAX_DEPTH) {
            throw cursor.error("nested deeper than " + MAX_DEPTH + " levels");
        }
        Value value = step.read();
        depth--;
        return value;
    }

    private Value list() throws TextSyntaxException {
        cursor.advance(1);
        List<Value> items = new ArrayList<>();
        while (true) {
            cursor.skipBlank();
            if (cursor.peek() == ']') {
                cursor.advance(1);
                return new ListValue(items);
            }
            items.add(value());
            cursor.skipBlank();
            if (!separator(']')) {
                return new ListValue(items);
            }
        }
    }

    private Value map() throws TextSyntaxException {
        cursor.advance(1);
        List<MapValue.Entry> entries = new ArrayList<>();
        Set<String> seen = new HashSet<>();
        while (true) {
            cursor.skipBlank();
            if (cursor.peek() == '}') {
                cursor.advance(1);
                return new MapValue(entries);
            }
            entries.add(entry(seen));
            cursor.skipBlank();
            if (!separator('}')) {
                return new MapValue(entries);
            }
        }
    }

    private MapValue.Entry entry(Set<String> seen) throws TextSyntaxException {
        int keyStart = cursor.index();
        String key = key();
        if (!seen.add(key)) {
            throw cursor.errorAt(keyStart, "`" + key + "` is already set in this object");
        }
        cursor.skipBlank();
        if (cursor.peek() != ':') {
            throw cursor.error("expected `:` after the key");
        }
        cursor.advance(1);
        cursor.skipBlank();
        return new MapValue.Entry(key, value());
    }

    /** After an item: {@code true} for a comma (more may follow), {@code false} for the closer. */
    private boolean separator(char closer) throws TextSyntaxException {
        int c = cursor.peek();
        if (c == ',') {
            cursor.advance(1);
            return true;
        }
        if (c == closer) {
            cursor.advance(1);
            return false;
        }
        throw cursor.error("expected `,` or `" + closer + "`");
    }

    private String key() throws TextSyntaxException {
        int first = cursor.peek();
        if (first == '"') {
            return StringLiteral.read(cursor);
        }
        if (!isNameStart(first)) {
            throw cursor.error("expected a key");
        }
        int start = cursor.index();
        while (isNamePart(cursor.peek())) {
            cursor.advance(1);
        }
        return cursor.text().substring(start, cursor.index());
    }

    private static boolean isNameStart(int c) {
        return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || c == '_' || c == '$';
    }

    private static boolean isNamePart(int c) {
        return isNameStart(c) || (c >= '0' && c <= '9');
    }
}
