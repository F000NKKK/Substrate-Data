package substrate.data;

/**
 * Frames: a value packed for a message whose schema both sides know. Nothing in
 * the bytes says what a value is or what it is called, so a frame is read by
 * its {@link ShapeDocument}.
 *
 * <pre>frame  = body, string table, table length (a varint written backwards)
 * record = varint length, its fields in declaration order</pre>
 */
public final class Frames {
    private Frames() {}

    /**
     * Reads {@code frame} by {@code shape}. Plain text comes out as the model
     * spells it (a leading {@code @} doubled), floats of 32 bits widened, an
     * absent optional as {@link Null}, and the trailing fields a shorter record
     * lacks left out of the map.
     *
     * @throws DecodeException when the bytes are not a frame of that shape
     */
    public static Value read(byte[] frame, ShapeDocument shape) throws DecodeException {
        return FrameDecoder.read(frame, shape);
    }
}
