package substrate.data;

/** Reads a JSON number at a cursor: an int when it is one that fits {@code i64}, else a float. */
final class NumberLiteral {
    private final TextCursor cursor;

    private NumberLiteral(TextCursor cursor) {
        this.cursor = cursor;
    }

    static Value read(TextCursor cursor) throws TextSyntaxException {
        return new NumberLiteral(cursor).run();
    }

    private Value run() throws TextSyntaxException {
        int start = cursor.index();
        if (cursor.peek() == '-') {
            cursor.advance(1);
        }
        int integerStart = cursor.index();
        if (digits() == 0) {
            throw cursor.error("expected digits");
        }
        if (cursor.text().charAt(integerStart) == '0' && cursor.index() - integerStart > 1) {
            throw cursor.errorAt(integerStart, "numbers don't start with 0");
        }
        boolean isFloat = fraction();
        isFloat |= exponent();
        String spelled = cursor.text().substring(start, cursor.index());
        if (!isFloat) {
            try {
                return new Int(Long.parseLong(spelled));
            } catch (NumberFormatException pastInt64) {
                // Past i64: read as the nearest float.
            }
        }
        return new Float(Double.parseDouble(spelled));
    }

    private boolean fraction() throws TextSyntaxException {
        if (cursor.peek() != '.') {
            return false;
        }
        cursor.advance(1);
        if (digits() == 0) {
            throw cursor.error("expected digits after `.`");
        }
        return true;
    }

    private boolean exponent() throws TextSyntaxException {
        if (cursor.peek() != 'e' && cursor.peek() != 'E') {
            return false;
        }
        cursor.advance(1);
        if (cursor.peek() == '+' || cursor.peek() == '-') {
            cursor.advance(1);
        }
        if (digits() == 0) {
            throw cursor.error("expected digits in the exponent");
        }
        return true;
    }

    private int digits() {
        int count = 0;
        while (cursor.peek() >= '0' && cursor.peek() <= '9') {
            cursor.advance(1);
            count++;
        }
        return count;
    }
}
