package substrate.data;

/** A value's first byte in the binary form: what it is. */
final class Tag {
    static final int NULL = 0;
    static final int FALSE = 1;
    static final int TRUE = 2;
    static final int INT = 3;
    static final int FLOAT = 4;
    static final int STR = 5;
    static final int LIST = 6;
    static final int MAP = 7;
    static final int REF = 8;
    static final int BYTES = 9;
    static final int INTS = 10;
    static final int STREAM = 11;
    static final int FLOAT32 = 12;
    static final int FLOATS = 13;
    static final int FLOATS32 = 14;

    private Tag() {}
}
