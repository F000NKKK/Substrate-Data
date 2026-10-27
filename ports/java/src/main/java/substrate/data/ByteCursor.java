package substrate.data;

import java.util.Arrays;

/** A read position over bytes; every read is checked and fails with a {@link DecodeException}. */
final class ByteCursor {
    private final byte[] bytes;
    private int position;

    ByteCursor(byte[] bytes) {
        this.bytes = bytes;
    }

    int position() {
        return position;
    }

    void seek(int to) {
        position = to;
    }

    int remaining() {
        return bytes.length - position;
    }

    boolean atEnd() {
        return position == bytes.length;
    }

    DecodeException error(DecodeErrorKind kind, String message) {
        return new DecodeException(kind, message, position);
    }

    int unsignedByte() throws DecodeException {
        if (position >= bytes.length) {
            throw error(DecodeErrorKind.UNEXPECTED_END, "unexpected end");
        }
        return bytes[position++] & 0xFF;
    }

    /** Exactly {@code count} bytes; {@code UNEXPECTED_END} when they are not there. */
    byte[] fixed(int count) throws DecodeException {
        if (count > remaining()) {
            throw error(DecodeErrorKind.UNEXPECTED_END, "unexpected end");
        }
        byte[] out = Arrays.copyOfRange(bytes, position, position + count);
        position += count;
        return out;
    }

    /** {@code length} bytes (unsigned); {@code BAD_LENGTH} when they are past the end. */
    byte[] take(long length) throws DecodeException {
        if (Long.compareUnsigned(length, remaining()) > 0) {
            throw error(DecodeErrorKind.BAD_LENGTH, "bytes past the end");
        }
        return fixed((int) length);
    }

    long varint() throws DecodeException {
        long value = 0;
        int shift = 0;
        for (int taken = 0; taken < Varint.MAX_BYTES; taken++) {
            if (atEnd()) {
                break;
            }
            int b = unsignedByte();
            long bits = b & Varint.LOW_BITS;
            if (shift == Varint.LAST_SHIFT && bits > 1) {
                break;
            }
            value |= bits << shift;
            if ((b & Varint.MORE) == 0) {
                return value;
            }
            shift += Varint.SHIFT;
        }
        throw error(DecodeErrorKind.BAD_VARINT, "a varint cut short or longer than 64 bits");
    }

    /** A count of things taking at least {@code min} bytes each, no bigger than what is left. */
    int count(int min) throws DecodeException {
        long n = varint();
        long left = remaining();
        if (Long.compareUnsigned(n, left / Math.max(min, 1)) > 0) {
            throw error(DecodeErrorKind.BAD_LENGTH,
                    "length " + Long.toUnsignedString(n) + " past the end (" + left + " bytes left)");
        }
        return (int) n;
    }
}
