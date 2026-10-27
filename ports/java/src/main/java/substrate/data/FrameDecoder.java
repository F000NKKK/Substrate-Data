package substrate.data;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.CharacterCodingException;
import java.nio.charset.CodingErrorAction;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

/** Reads a frame by its shape. */
final class FrameDecoder {
    private static final int F64_BYTES = 8;
    private static final int F32_BYTES = 4;

    private final ShapeDocument document;
    private final ByteCursor body;
    private final ByteCursor table;
    private final List<String> strings = new ArrayList<>();
    private int depth;

    private FrameDecoder(ShapeDocument document, byte[] body, byte[] table) {
        this.document = document;
        this.body = new ByteCursor(body);
        this.table = new ByteCursor(table);
    }

    static Value read(byte[] frame, ShapeDocument document) throws DecodeException {
        long[] trailer = tableLength(frame);
        long tableLength = trailer[0];
        long trailerBytes = trailer[1];
        long bodyEnd = frame.length - trailerBytes - tableLength;
        if (tableLength < 0 || bodyEnd < 0) {
            throw bad("the string table is longer than the frame");
        }
        FrameDecoder decoder = new FrameDecoder(document,
                Arrays.copyOfRange(frame, 0, (int) bodyEnd),
                Arrays.copyOfRange(frame, (int) bodyEnd, (int) (frame.length - trailerBytes)));
        Value value = decoder.value(document.root());
        if (!decoder.body.atEnd()) {
            throw decoder.body.error(DecodeErrorKind.TRAILING_BYTES, "bytes after the value");
        }
        return value;
    }

    private static DecodeException bad(String message) {
        return new DecodeException(DecodeErrorKind.BAD_FRAME, message, 0);
    }

    /** The table length, written as a varint backwards from the frame's end: its value, and its byte count. */
    private static long[] tableLength(byte[] frame) throws DecodeException {
        long value = 0;
        int shift = 0;
        for (int taken = 1; taken <= frame.length; taken++) {
            if (shift >= Varint.MAX_BYTES * Varint.SHIFT) {
                break;
            }
            int b = frame[frame.length - taken] & 0xFF;
            long bits = b & Varint.LOW_BITS;
            if (shift == Varint.LAST_SHIFT && bits > 1) {
                break;
            }
            value |= bits << shift;
            if ((b & Varint.MORE) == 0) {
                return new long[] {value, taken};
            }
            shift += Varint.SHIFT;
        }
        throw bad("no table length at the end of the frame");
    }

    private Value value(Shape shape) throws DecodeException {
        return switch (shape) {
            case Shape.BoolShape s -> new Bool(bool());
            case Shape.IntShape s -> new Int(Varint.unzigzag(body.varint()));
            case Shape.FloatShape s -> new Float(s.bits() == 32 ? f32() : f64());
            case Shape.StringShape s -> new Str(Typed.escapeAt(string()));
            case Shape.BytesShape s -> new Bytes(body.take(body.count(1)));
            case Shape.OptionalShape s -> bool() ? value(s.of()) : Null.INSTANCE;
            case Shape.ListShape s -> list(s.of());
            case Shape.RefShape s -> reference(s);
            case Shape.StructShape s -> record(s);
            case Shape.DefShape s -> value(definition(s));
            case Shape.EnumShape s when !s.tagged() -> variant(s);
            case Shape.EnumShape s -> tagged();
            case Shape.OtherShape s -> tagged();
        };
    }

    private Shape definition(Shape.DefShape def) throws DecodeException {
        Shape shape = document.defs().get(def.name());
        if (shape == null) {
            throw bad("no definition `" + def.name() + "` in the shape");
        }
        return shape;
    }

    private boolean bool() throws DecodeException {
        int b = body.unsignedByte();
        if (b > 1) {
            throw body.error(DecodeErrorKind.BAD_FRAME, "a bool byte is 0 or 1");
        }
        return b == 1;
    }

    private double f32() throws DecodeException {
        return ByteBuffer.wrap(body.fixed(F32_BYTES)).order(ByteOrder.LITTLE_ENDIAN).getFloat();
    }

    private double f64() throws DecodeException {
        return ByteBuffer.wrap(body.fixed(F64_BYTES)).order(ByteOrder.LITTLE_ENDIAN).getDouble();
    }

    /** A string: its index in the table, which is read as far as it is needed. */
    private String string() throws DecodeException {
        long index = body.varint();
        if (Long.compareUnsigned(index, Integer.MAX_VALUE) > 0) {
            throw body.error(DecodeErrorKind.BAD_STRING_INDEX, "string index past the table");
        }
        while (strings.size() <= index) {
            strings.add(nextTableString());
        }
        return strings.get((int) index);
    }

    private String nextTableString() throws DecodeException {
        byte[] raw = table.take(table.varint());
        try {
            return StandardCharsets.UTF_8.newDecoder()
                    .onMalformedInput(CodingErrorAction.REPORT)
                    .onUnmappableCharacter(CodingErrorAction.REPORT)
                    .decode(ByteBuffer.wrap(raw)).toString();
        } catch (CharacterCodingException e) {
            throw table.error(DecodeErrorKind.BAD_UTF8, "a string isn't UTF-8");
        }
    }

    /** A reference of declared kinds packs only its id; one of any kind, its kind and id as text. */
    private Value reference(Shape.RefShape shape) throws DecodeException {
        String kind = shape.kinds().isEmpty() ? string() : shape.kinds().get(0);
        String id = shape.id() instanceof Shape.IntShape ? Long.toString(Varint.unzigzag(body.varint())) : string();
        if (!Ref.isValidKind(kind) || !Ref.isValidId(id)) {
            throw body.error(DecodeErrorKind.BAD_REFERENCE, "`@" + kind + ":" + id + "` isn't a reference");
        }
        return new Ref(kind, id);
    }

    private Value variant(Shape.EnumShape shape) throws DecodeException {
        long index = body.varint();
        if (Long.compareUnsigned(index, shape.variants().size()) >= 0) {
            throw body.error(DecodeErrorKind.BAD_FRAME, "variant " + Long.toUnsignedString(index) + " isn't in the enum");
        }
        return new Str(shape.variants().get((int) index));
    }

    /** A value held in its binary form: its bytes, with a string table of their own. */
    private Value tagged() throws DecodeException {
        return Binary.decodeBody(body.take(body.count(1)));
    }

    private Value list(Shape item) throws DecodeException {
        Shape resolved = item instanceof Shape.DefShape def ? definition(def) : item;
        return switch (resolved) {
            case Shape.IntShape s -> ints(s);
            case Shape.FloatShape s when s.bits() == 32 -> floats(false);
            case Shape.FloatShape s -> floats(true);
            default -> items(item);
        };
    }

    private Value items(Shape item) throws DecodeException {
        int n = body.count(1);
        int byteLength = body.count(1);
        int end = body.position() + byteLength;
        List<Value> out = new ArrayList<>(Math.min(n, 1024));
        for (int i = 0; i < n; i++) {
            out.add(value(item));
        }
        if (body.position() > end) {
            throw body.error(DecodeErrorKind.BAD_FRAME, "a list's items run past its length");
        }
        body.seek(end);
        return new ListValue(out);
    }

    /** Bit-packed numbers: a count, a width, and the lanes. */
    private Value ints(Shape.IntShape shape) throws DecodeException {
        long count = body.varint();
        int width = body.unsignedByte();
        if (!IntLanes.isValidWidth(width) || Long.compareUnsigned(count, (long) body.remaining() * 8 / width) > 0) {
            throw body.error(DecodeErrorKind.BAD_FRAME, "a packed list of ints that doesn't fit its bytes");
        }
        int n = (int) count;
        long[] numbers = IntLanes.unpack(body.take(IntLanes.packedBytes(n, width)), n, width);
        List<Value> out = new ArrayList<>(n);
        for (long v : numbers) {
            out.add(new Int(intOf(shape, v)));
        }
        return new ListValue(out);
    }

    private long intOf(Shape.IntShape shape, long v) throws DecodeException {
        if (shape.min() < 0) {
            return Varint.unzigzag(v);
        }
        // A 64-bit unsigned list keeps its bits as the model's Int does; a narrower one only holds what fits.
        if (shape.max() != Long.MAX_VALUE && v < 0) {
            throw body.error(DecodeErrorKind.BAD_FRAME, "a number that doesn't fit an int");
        }
        return v;
    }

    /** A list of floats: a count, for f64 a flag (1: every item is an f32 exactly), the raw values. */
    private Value floats(boolean wide) throws DecodeException {
        int n = body.count(1);
        boolean narrow = !wide || bool();
        int width = narrow ? F32_BYTES : F64_BYTES;
        ByteBuffer raw = ByteBuffer.wrap(body.take((long) n * width)).order(ByteOrder.LITTLE_ENDIAN);
        List<Value> out = new ArrayList<>(n);
        for (int i = 0; i < n; i++) {
            out.add(new Float(narrow ? raw.getFloat() : raw.getDouble()));
        }
        return new ListValue(out);
    }

    /** A record: its fields in order by name; fields the shape doesn't know are skipped, missing ones left out. */
    private Value record(Shape.StructShape shape) throws DecodeException {
        int length = body.count(1);
        int end = body.position() + length;
        depth++;
        if (depth > Binary.MAX_DEPTH) {
            throw body.error(DecodeErrorKind.TOO_DEEP, "nested deeper than " + Binary.MAX_DEPTH + " levels");
        }
        List<MapValue.Entry> entries = new ArrayList<>();
        for (Shape.Field field : shape.fields()) {
            if (body.position() >= end) {
                break;
            }
            entries.add(new MapValue.Entry(field.name(), value(field.shape())));
        }
        depth--;
        if (body.position() > end) {
            throw body.error(DecodeErrorKind.BAD_FRAME, "a record's fields run past its length");
        }
        body.seek(end);
        return new MapValue(entries);
    }
}
