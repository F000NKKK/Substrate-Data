package substrate.data;

/** A position in the text being parsed, with the way to report an error at it. */
final class TextCursor {
    private static final char BYTE_ORDER_MARK = '﻿';
    private static final int NONE = -1;

    private final String text;
    private int index;

    TextCursor(String text) {
        this.text = text;
        this.index = !text.isEmpty() && text.charAt(0) == BYTE_ORDER_MARK ? 1 : 0;
    }

    String text() {
        return text;
    }

    int index() {
        return index;
    }

    void advance(int count) {
        index += count;
    }

    boolean atEnd() {
        return index >= text.length();
    }

    /** The current character, or -1 at the end. */
    int peek() {
        return peekAt(0);
    }

    int peekAt(int ahead) {
        int at = index + ahead;
        return at < text.length() ? text.charAt(at) : NONE;
    }

    boolean startsWith(String prefix) {
        return text.startsWith(prefix, index);
    }

    /** Skips whitespace and {@code //} and {@code /* *}{@code /} comments. */
    void skipBlank() throws TextSyntaxException {
        while (true) {
            while (isSpace(peek())) {
                index++;
            }
            if (peek() != '/') {
                return;
            }
            if (peekAt(1) == '/') {
                int end = text.indexOf('\n', index);
                index = end < 0 ? text.length() : end;
            } else if (peekAt(1) == '*') {
                skipBlockComment();
            } else {
                return;
            }
        }
    }

    TextSyntaxException error(String message) {
        return errorAt(index, message);
    }

    TextSyntaxException errorAt(int offset, String message) {
        String full = offset >= text.length() ? message + ", found the end of the file" : message;
        int line = 1;
        int lineStart = 0;
        for (int i = 0; i < offset && i < text.length(); i++) {
            if (text.charAt(i) == '\n') {
                line++;
                lineStart = i + 1;
            }
        }
        int end = Math.min(offset, text.length());
        int column = text.codePointCount(lineStart, end) + 1;
        return new TextSyntaxException(full, line, column);
    }

    private void skipBlockComment() throws TextSyntaxException {
        int start = index;
        int end = text.indexOf("*/", index + 2);
        if (end < 0) {
            throw errorAt(start, "unclosed `/*` comment");
        }
        index = end + 2;
    }

    private static boolean isSpace(int c) {
        return c == ' ' || c == '\t' || c == '\n' || c == '\r';
    }
}
