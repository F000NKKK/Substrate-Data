package substrate.data;

/**
 * Numbers bit-packed in lanes: what a frame's list of ints is made of.
 *
 * <p>A list of {@code n} numbers of {@code width} bits (1 to 32) is {@code n / 256}
 * groups, then the rest packed one after another, little-endian bits. A group is
 * 256 numbers in 8 lanes of 32-bit words: number {@code j} goes to lane {@code j % 8},
 * the {@code j / 8}-th in it; a lane's numbers are packed one after another into
 * its words, and word {@code k} of lane {@code l} is the group's {@code k * 8 + l}-th.
 * Numbers wider than 32 bits go one after another, with no groups.
 */
final class IntLanes {
    static final int LANES = 8;
    static final int GROUP = 256;
    static final int GROUP_MAX_BITS = 32;
    private static final int WORD_BYTES = 4;
    private static final int WORD_BITS = 32;
    private static final int MAX_WIDTH = 64;

    private IntLanes() {}

    /** Whether {@code width} is a width a list may be packed with. */
    static boolean isValidWidth(int width) {
        return width >= 1 && width <= MAX_WIDTH;
    }

    /** The bytes {@code n} numbers of {@code width} bits take. */
    static long packedBytes(long n, int width) {
        long groups = width <= GROUP_MAX_BITS ? n / GROUP : 0;
        long rest = n - groups * GROUP;
        return groups * groupBytes(width) + (rest * width + 7) / 8;
    }

    private static int groupBytes(int width) {
        return LANES * width * WORD_BYTES;
    }

    /** The {@code n} numbers in {@code raw} (already checked to hold {@link #packedBytes}). */
    static long[] unpack(byte[] raw, int n, int width) {
        long[] out = new long[n];
        int groups = width <= GROUP_MAX_BITS ? n / GROUP : 0;
        for (int g = 0; g < groups; g++) {
            for (int j = 0; j < GROUP; j++) {
                out[g * GROUP + j] = inGroup(raw, g * groupBytes(width), j, width);
            }
        }
        int tailAt = groups * groupBytes(width);
        for (int i = groups * GROUP; i < n; i++) {
            out[i] = sequential(raw, tailAt, i - groups * GROUP, width);
        }
        return out;
    }

    private static long inGroup(byte[] raw, int groupAt, int j, int width) {
        int lane = j % LANES;
        int bit = (j / LANES) * width;
        int word = bit / WORD_BITS;
        int shift = bit % WORD_BITS;
        long two = wordAt(raw, groupAt, word, lane);
        if (shift + width > WORD_BITS) {
            two |= wordAt(raw, groupAt, word + 1, lane) << WORD_BITS;
        }
        return (two >>> shift) & ((1L << width) - 1);
    }

    private static long wordAt(byte[] raw, int groupAt, int word, int lane) {
        int at = groupAt + (word * LANES + lane) * WORD_BYTES;
        return (raw[at] & 0xFFL) | (raw[at + 1] & 0xFFL) << 8 | (raw[at + 2] & 0xFFL) << 16 | (raw[at + 3] & 0xFFL) << 24;
    }

    private static long sequential(byte[] raw, int tailAt, int index, int width) {
        long bitPos = (long) index * width;
        long value = 0;
        int got = 0;
        while (got < width) {
            int at = tailAt + (int) (bitPos >>> 3);
            int offset = (int) (bitPos & 7);
            int take = Math.min(8 - offset, width - got);
            long bits = ((raw[at] & 0xFF) >>> offset) & ((1 << take) - 1);
            value |= bits << got;
            got += take;
            bitPos += take;
        }
        return value;
    }
}
