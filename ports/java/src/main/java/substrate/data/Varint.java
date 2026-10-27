package substrate.data;

import java.io.ByteArrayOutputStream;

/** Varints (LEB128 of a u64) and zigzag. */
final class Varint {
    static final int LOW_BITS = 0x7F;
    static final int MORE = 0x80;
    static final int SHIFT = 7;
    /** The most bytes a u64 takes. */
    static final int MAX_BYTES = 10;
    /** The shift of the tenth byte: the one bit left of 64. */
    static final int LAST_SHIFT = 63;

    private Varint() {}

    static void write(ByteArrayOutputStream out, long value) {
        long rest = value;
        while (Long.compareUnsigned(rest, LOW_BITS) > 0) {
            out.write((int) (rest & LOW_BITS) | MORE);
            rest >>>= SHIFT;
        }
        out.write((int) rest);
    }

    static long zigzag(long n) {
        return (n << 1) ^ (n >> 63);
    }

    static long unzigzag(long z) {
        return (z >>> 1) ^ -(z & 1);
    }
}
