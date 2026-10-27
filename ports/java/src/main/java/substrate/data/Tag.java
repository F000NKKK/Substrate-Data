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
    /** Version 2: a map whose keys are a shape, then one value per key. */
    static final int MAP_SHAPE = 15;
    /** Version 2: a list of maps of one shape: a count, the shape, the values. */
    static final int RECORDS = 16;
    /** Version 2: the first of 128 tags that are an int by themselves. */
    static final int INT_INLINE = 128;
    /** Version 2: the first of 32 tags that are a table string by themselves. */
    static final int STR_INDEX = 32;
    /** How many strings a tag of its own names. */
    static final int STR_INDEX_COUNT = 32;
    /** The smallest int a tag of its own is. */
    static final int INLINE_INT_MIN = -16;
    /** The largest int a tag of its own is. */
    static final int INLINE_INT_MAX = 111;

    private Tag() {}
}
