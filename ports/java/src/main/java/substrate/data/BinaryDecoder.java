package substrate.data;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.CharacterCodingException;
import java.nio.charset.CodingErrorAction;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashSet;
import java.util.List;
import java.util.Set;

/** Reads the binary form into a {@link Value}; every count and length is checked before it is used. */
final class BinaryDecoder {
    private static final int F64_BYTES = 8;
    private static final int F32_BYTES = 4;
    private static final int MAP_ENTRY_MIN = 2;
    private static final int PREALLOCATE_MAX = 1024;

    private final ByteCursor cursor;
    private final List<String> strings = new ArrayList<>();
    /** The map shapes read so far (version 2): each its keys. */
    private final List<List<String>> shapes = new ArrayList<>();
    private final boolean v2;
    private int depth;

    private BinaryDecoder(ByteCursor cursor, boolean v2, int depth) {
        this.cursor = cursor;
        this.v2 = v2;
        this.depth = depth;
    }

    /** A whole document: header, root value, and nothing after it. */
    static Value document(byte[] bytes) throws DecodeException {
        ByteCursor cursor = new ByteCursor(bytes);
        boolean v2 = header(cursor);
        BinaryDecoder decoder = new BinaryDecoder(cursor, v2, 0);
        Value value = decoder.value();
        decoder.end();
        return value;
    }

    /** A value held in its binary form with no header (its own string table), and nothing after it. */
    static Value body(byte[] bytes) throws DecodeException {
        BinaryDecoder decoder = new BinaryDecoder(new ByteCursor(bytes), true, 0);
        Value value = decoder.value();
        decoder.end();
        return value;
    }

    /** The header: whether the document is version 2 (else version 1). */
    private static boolean header(ByteCursor cursor) throws DecodeException {
        byte[] magic = Binary.MAGIC;
        if (cursor.remaining() < magic.length || !Arrays.equals(cursor.fixed(magic.length), magic)) {
            cursor.seek(0);
            throw cursor.error(DecodeErrorKind.BAD_MAGIC, "not Substrate Data (no `SDAT` header)");
        }
        int version = cursor.unsignedByte();
        if (version != Binary.VERSION && version != Binary.LEGACY_VERSION) {
            throw cursor.error(DecodeErrorKind.BAD_VERSION, "Substrate Data version " + version
                    + ", this reads versions " + Binary.LEGACY_VERSION + " and " + Binary.VERSION);
        }
        return version == Binary.VERSION;
    }

    private void end() throws DecodeException {
        if (!cursor.atEnd()) {
            throw cursor.error(DecodeErrorKind.TRAILING_BYTES, "bytes after the value");
        }
    }

    private Value value() throws DecodeException {
        int tag = cursor.unsignedByte();
        return switch (tag) {
            case Tag.NULL -> Null.INSTANCE;
            case Tag.FALSE -> new Bool(false);
            case Tag.TRUE -> new Bool(true);
            case Tag.INT -> new Int(Varint.unzigzag(cursor.varint()));
            case Tag.FLOAT -> new Float(littleEndian(cursor.fixed(F64_BYTES)).getDouble());
            case Tag.FLOAT32 -> new Float(littleEndian(cursor.fixed(F32_BYTES)).getFloat());
            case Tag.STR -> new Str(string());
            case Tag.LIST -> list();
            case Tag.MAP -> map();
            case Tag.REF -> reference();
            case Tag.BYTES -> new Bytes(cursor.take(cursor.varint()));
            case Tag.INTS -> ints();
            case Tag.STREAM -> stream();
            case Tag.FLOATS -> floats(F64_BYTES);
            case Tag.FLOATS32 -> floats(F32_BYTES);
            default -> version2(tag);
        };
    }

    /** A tag only version 2 has: shapes, records, an int or a string by itself. */
    private Value version2(int tag) throws DecodeException {
        if (v2) {
            if (tag == Tag.MAP_SHAPE) {
                return shapedMap(shapeSpec());
            }
            if (tag == Tag.RECORDS) {
                return records();
            }
            if (tag >= Tag.INT_INLINE) {
                return new Int(tag - Tag.INT_INLINE + Tag.INLINE_INT_MIN);
            }
            if (tag >= Tag.STR_INDEX && tag < Tag.STR_INDEX + Tag.STR_INDEX_COUNT) {
                return new Str(known(tag - Tag.STR_INDEX));
            }
        }
        cursor.seek(cursor.position() - 1);
        throw cursor.error(DecodeErrorKind.UNKNOWN_TAG, "unknown tag " + tag);
    }

    /** A shape: its number, or its keys the first time: its keys. */
    private List<String> shapeSpec() throws DecodeException {
        long marker = cursor.varint();
        long n = marker >>> 1;
        if ((marker & Binary.NEW_STRING) == 0) {
            if (Long.compareUnsigned(n, shapes.size()) >= 0) {
                throw cursor.error(DecodeErrorKind.BAD_SHAPE_INDEX,
                        "shape " + Long.toUnsignedString(n) + " isn't defined");
            }
            return shapes.get((int) n);
        }
        if (n == 0) {
            throw cursor.error(DecodeErrorKind.BAD_LENGTH, "a shape with no keys");
        }
        if (Long.compareUnsigned(n, cursor.remaining()) > 0) {
            throw cursor.error(DecodeErrorKind.BAD_LENGTH, "length " + Long.toUnsignedString(n)
                    + " past the end (" + cursor.remaining() + " bytes left)");
        }
        List<String> keys = new ArrayList<>((int) n);
        Set<String> seen = new HashSet<>();
        for (long i = 0; i < n; i++) {
            int at = cursor.position();
            String key = string();
            if (!seen.add(key)) {
                cursor.seek(at);
                throw cursor.error(DecodeErrorKind.DUPLICATE_KEY, "`" + key + "` is already set in this map");
            }
            keys.add(key);
        }
        shapes.add(keys);
        return keys;
    }

    private Value shapedMap(List<String> keys) throws DecodeException {
        if (keys.size() > cursor.remaining()) {
            throw cursor.error(DecodeErrorKind.BAD_LENGTH, "length " + keys.size()
                    + " past the end (" + cursor.remaining() + " bytes left)");
        }
        deeper();
        Value map = shapedEntries(keys);
        depth--;
        return map;
    }

    private Value shapedEntries(List<String> keys) throws DecodeException {
        List<MapValue.Entry> entries = new ArrayList<>(keys.size());
        for (String key : keys) {
            entries.add(new MapValue.Entry(key, value()));
        }
        return new MapValue(entries);
    }

    /** A list of maps of one shape: a count, the shape, the values. */
    private Value records() throws DecodeException {
        int n = cursor.count(1);
        List<String> keys = shapeSpec();
        if (n > cursor.remaining() / keys.size()) {
            throw cursor.error(DecodeErrorKind.BAD_LENGTH,
                    "length " + n + " past the end (" + cursor.remaining() + " bytes left)");
        }
        deeper();
        List<Value> items = new ArrayList<>(Math.min(n, PREALLOCATE_MAX));
        for (int i = 0; i < n; i++) {
            deeper();
            items.add(shapedEntries(keys));
            depth--;
        }
        depth--;
        return new ListValue(items);
    }

    private Value list() throws DecodeException {
        int n = cursor.count(1);
        deeper();
        List<Value> items = new ArrayList<>(Math.min(n, PREALLOCATE_MAX));
        for (int i = 0; i < n; i++) {
            items.add(value());
        }
        depth--;
        return new ListValue(items);
    }

    private Value map() throws DecodeException {
        int n = cursor.count(MAP_ENTRY_MIN);
        deeper();
        List<MapValue.Entry> entries = new ArrayList<>(Math.min(n, PREALLOCATE_MAX));
        Set<String> seen = new HashSet<>();
        for (int i = 0; i < n; i++) {
            int at = cursor.position();
            String key = string();
            if (!seen.add(key)) {
                cursor.seek(at);
                throw cursor.error(DecodeErrorKind.DUPLICATE_KEY, "`" + key + "` is already set in this map");
            }
            entries.add(new MapValue.Entry(key, value()));
        }
        depth--;
        return new MapValue(entries);
    }

    private Value reference() throws DecodeException {
        int kindAt = cursor.position();
        String kind = string();
        int idAt = cursor.position();
        String id = string();
        if (!Ref.isValidKind(kind)) {
            cursor.seek(kindAt);
            throw cursor.error(DecodeErrorKind.BAD_REFERENCE, "`" + kind + "` isn't a kind");
        }
        if (!Ref.isValidId(id)) {
            cursor.seek(idAt);
            throw cursor.error(DecodeErrorKind.BAD_REFERENCE, "`" + id + "` isn't a reference's id");
        }
        return new Ref(kind, id);
    }

    private Value ints() throws DecodeException {
        int n = cursor.count(1);
        List<Value> items = new ArrayList<>(Math.min(n, PREALLOCATE_MAX));
        for (int i = 0; i < n; i++) {
            items.add(new Int(Varint.unzigzag(cursor.varint())));
        }
        return new ListValue(items);
    }

    private Value floats(int width) throws DecodeException {
        int n = cursor.count(width);
        ByteBuffer raw = littleEndian(cursor.take((long) n * width));
        List<Value> items = new ArrayList<>(n);
        for (int i = 0; i < n; i++) {
            items.add(new Float(width == F64_BYTES ? raw.getDouble() : raw.getFloat()));
        }
        return new ListValue(items);
    }

    /** A stream: its byte length, then a body with a string table of its own. */
    private Value stream() throws DecodeException {
        long length = littleEndian(cursor.fixed(Binary.STREAM_LEN_BYTES)).getInt() & 0xFFFFFFFFL;
        ByteCursor inner = new ByteCursor(cursor.take(length));
        BinaryDecoder decoder = new BinaryDecoder(inner, v2, depth);
        int n = inner.count(1);
        List<Value> items = new ArrayList<>(Math.min(n, PREALLOCATE_MAX));
        for (int i = 0; i < n; i++) {
            items.add(decoder.value());
        }
        decoder.end();
        return new ListValue(items);
    }

    /** A string: new (read, and added to the table) or the table's. */
    private String string() throws DecodeException {
        long v = cursor.varint();
        long index = v >>> 1;
        if ((v & Binary.NEW_STRING) == 0) {
            if (Long.compareUnsigned(index, strings.size()) >= 0) {
                throw cursor.error(DecodeErrorKind.BAD_STRING_INDEX,
                        "string " + Long.toUnsignedString(index) + " isn't in the table");
            }
            return strings.get((int) index);
        }
        String s = utf8(cursor.take(index));
        strings.add(s);
        return s;
    }

    /** The string numbered {@code index} in the table. */
    private String known(int index) throws DecodeException {
        if (index >= strings.size()) {
            throw cursor.error(DecodeErrorKind.BAD_STRING_INDEX, "string " + index + " isn't in the table");
        }
        return strings.get(index);
    }

    private String utf8(byte[] raw) throws DecodeException {
        try {
            return StandardCharsets.UTF_8.newDecoder()
                    .onMalformedInput(CodingErrorAction.REPORT)
                    .onUnmappableCharacter(CodingErrorAction.REPORT)
                    .decode(ByteBuffer.wrap(raw)).toString();
        } catch (CharacterCodingException e) {
            throw cursor.error(DecodeErrorKind.BAD_UTF8, "a string isn't UTF-8");
        }
    }

    private void deeper() throws DecodeException {
        depth++;
        if (depth > Binary.MAX_DEPTH) {
            throw cursor.error(DecodeErrorKind.TOO_DEEP, "nested deeper than " + Binary.MAX_DEPTH + " levels");
        }
    }

    private static ByteBuffer littleEndian(byte[] bytes) {
        return ByteBuffer.wrap(bytes).order(ByteOrder.LITTLE_ENDIAN);
    }
}
