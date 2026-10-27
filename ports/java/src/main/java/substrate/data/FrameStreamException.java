package substrate.data;

import java.io.IOException;

/** A byte stream of frames that can't be read or written. */
public final class FrameStreamException extends IOException {
    private static final long serialVersionUID = 1L;

    /** What went wrong. */
    public enum Kind {
        /** The stream ends inside a length or a frame. */
        UNEXPECTED_EOF,
        /** A length above {@link FrameStream#MAX_FRAME}. */
        INVALID_DATA,
        /** A frame over {@link FrameStream#MAX_FRAME}, refused before anything is written. */
        INVALID_INPUT
    }

    private final Kind kind;

    public FrameStreamException(Kind kind, String message) {
        super(message);
        this.kind = kind;
    }

    /** What went wrong. */
    public Kind kind() {
        return kind;
    }
}
