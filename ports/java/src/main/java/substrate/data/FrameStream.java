package substrate.data;

import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.Optional;

/**
 * Messages on a byte stream (a socket, a pipe): each a little-endian {@code u32}
 * length (at most {@link #MAX_FRAME}), then the message's bytes, a binary
 * document. A stream ends cleanly only between frames; one cut inside a length
 * or a body is an {@link FrameStreamException.Kind#UNEXPECTED_EOF}.
 */
public final class FrameStream {
    /** The longest frame read: anything longer is a broken or hostile peer. */
    public static final int MAX_FRAME = 256 << 20;

    private static final int LENGTH_BYTES = 4;

    private FrameStream() {}

    /**
     * The next frame's bytes; empty where the stream ends between frames. The body
     * is read as it arrives, never allocated by the length a peer claims.
     *
     * @throws FrameStreamException for a stream that ends inside a frame, or a length over {@link #MAX_FRAME}
     * @throws IOException the stream's own error
     */
    public static Optional<byte[]> read(InputStream in) throws IOException {
        int first = in.read();
        if (first < 0) {
            return Optional.empty();
        }
        byte[] rest = in.readNBytes(LENGTH_BYTES - 1);
        if (rest.length < LENGTH_BYTES - 1) {
            throw new FrameStreamException(FrameStreamException.Kind.UNEXPECTED_EOF, "the stream ends inside a length");
        }
        long length = (first & 0xFFL) | (rest[0] & 0xFFL) << 8 | (rest[1] & 0xFFL) << 16 | (rest[2] & 0xFFL) << 24;
        if (length > MAX_FRAME) {
            throw new FrameStreamException(FrameStreamException.Kind.INVALID_DATA, "frame too long");
        }
        byte[] bytes = in.readNBytes((int) length);
        if (bytes.length < length) {
            throw new FrameStreamException(FrameStreamException.Kind.UNEXPECTED_EOF, "the stream ends inside a frame");
        }
        return Optional.of(bytes);
    }

    /**
     * Writes {@code bytes} (a message's binary form) as one frame, in one write, and flushes.
     *
     * @throws FrameStreamException for a frame over {@link #MAX_FRAME}
     * @throws IOException the stream's own error
     */
    public static void write(OutputStream out, byte[] bytes) throws IOException {
        if (bytes.length > MAX_FRAME) {
            throw new FrameStreamException(FrameStreamException.Kind.INVALID_INPUT, "frame too long");
        }
        ByteBuffer frame = ByteBuffer.allocate(LENGTH_BYTES + bytes.length).order(ByteOrder.LITTLE_ENDIAN);
        frame.putInt(bytes.length).put(bytes);
        out.write(frame.array());
        out.flush();
    }
}
