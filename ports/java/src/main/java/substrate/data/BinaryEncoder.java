package substrate.data;

import java.io.ByteArrayOutputStream;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/**
 * Writes a {@link Value} in the binary form, version 2: strings interned and
 * written where first used; a map's keys are a shape, written once; a list of
 * ints or floats is packed; a list of maps with one set of keys is records;
 * a small int or an early string is one byte. The form is chosen from the
 * value alone, so every writer writes the same bytes for it.
 */
final class BinaryEncoder {
    private final ByteArrayOutputStream out = new ByteArrayOutputStream();
    private final Map<String, Integer> strings = new HashMap<>();
    /** Shapes by the table numbers of their keys. */
    private final Map<List<Integer>, Integer> shapes = new HashMap<>();

    private BinaryEncoder() {}

    static byte[] document(Value value) {
        BinaryEncoder encoder = new BinaryEncoder();
        encoder.out.writeBytes(Binary.MAGIC);
        encoder.out.write(Binary.VERSION);
        encoder.value(value);
        return encoder.out.toByteArray();
    }

    private void value(Value value) {
        switch (value) {
            case Null n -> out.write(Tag.NULL);
            case Bool b -> out.write(b.value() ? Tag.TRUE : Tag.FALSE);
            case Int i -> integer(i.value());
            case Float f -> floating(f.bits());
            case Str s -> str(s.value());
            case ListValue l -> list(l);
            case MapValue m -> map(m);
            case Ref r -> {
                out.write(Tag.REF);
                string(r.kind());
                string(r.id());
            }
            case Bytes b -> {
                byte[] data = b.data();
                out.write(Tag.BYTES);
                Varint.write(out, data.length);
                out.writeBytes(data);
            }
        }
    }

    /** An int: one byte for -16 to 111, else a tag and a varint. */
    private void integer(long n) {
        if (n >= Tag.INLINE_INT_MIN && n <= Tag.INLINE_INT_MAX) {
            out.write(Tag.INT_INLINE + (int) (n - Tag.INLINE_INT_MIN));
            return;
        }
        out.write(Tag.INT);
        Varint.write(out, Varint.zigzag(n));
    }

    /** A float: four bytes when it is an {@code f32} exactly, else eight. */
    private void floating(long bits) {
        if (narrows(bits)) {
            out.write(Tag.FLOAT32);
            writeNarrow(bits);
            return;
        }
        out.write(Tag.FLOAT);
        writeWide(bits);
    }

    /** Whether the float with these bits is an {@code f32} exactly, bit for bit (a NaN never is). */
    private static boolean narrows(long bits) {
        double wide = Double.longBitsToDouble(bits);
        return !Double.isNaN(wide) && Double.doubleToRawLongBits((double) (float) wide) == bits;
    }

    private void writeNarrow(long bits) {
        int narrow = java.lang.Float.floatToRawIntBits((float) Double.longBitsToDouble(bits));
        out.writeBytes(ByteBuffer.allocate(java.lang.Float.BYTES).order(ByteOrder.LITTLE_ENDIAN).putInt(narrow).array());
    }

    private void writeWide(long bits) {
        out.writeBytes(ByteBuffer.allocate(Double.BYTES).order(ByteOrder.LITTLE_ENDIAN).putLong(bits).array());
    }

    /** A string value: a tag of its own when it is one of the first 32 of the table. */
    private void str(String text) {
        Integer index = strings.get(text);
        if (index != null && index < Tag.STR_INDEX_COUNT) {
            out.write(Tag.STR_INDEX + index);
            return;
        }
        out.write(Tag.STR);
        string(text);
    }

    private void list(ListValue list) {
        List<Value> items = list.items();
        if (!items.isEmpty() && items.stream().allMatch(v -> v instanceof Int)) {
            out.write(Tag.INTS);
            Varint.write(out, items.size());
            items.forEach(v -> Varint.write(out, Varint.zigzag(((Int) v).value())));
        } else if (!items.isEmpty() && items.stream().allMatch(v -> v instanceof Float)) {
            floats(items);
        } else if (isRecords(items)) {
            records(items);
        } else {
            out.write(Tag.LIST);
            Varint.write(out, items.size());
            items.forEach(this::value);
        }
    }

    private void floats(List<Value> items) {
        boolean narrow = items.stream().allMatch(v -> narrows(((Float) v).bits()));
        out.write(narrow ? Tag.FLOATS32 : Tag.FLOATS);
        Varint.write(out, items.size());
        for (Value v : items) {
            long bits = ((Float) v).bits();
            if (narrow) {
                writeNarrow(bits);
            } else {
                writeWide(bits);
            }
        }
    }

    /** Whether every item is a map with the first one's keys, in its order (and it has some). */
    private static boolean isRecords(List<Value> items) {
        if (items.isEmpty() || !(items.get(0) instanceof MapValue first) || first.size() == 0) {
            return false;
        }
        return items.stream().allMatch(v -> v instanceof MapValue m && keysOf(m).equals(keysOf(first)));
    }

    private static List<String> keysOf(MapValue map) {
        return map.entries().stream().map(MapValue.Entry::key).toList();
    }

    private void records(List<Value> items) {
        out.write(Tag.RECORDS);
        Varint.write(out, items.size());
        shape((MapValue) items.get(0));
        for (Value item : items) {
            values((MapValue) item);
        }
    }

    private void map(MapValue map) {
        if (map.size() == 0) {
            out.write(Tag.MAP);
            Varint.write(out, 0);
            return;
        }
        out.write(Tag.MAP_SHAPE);
        shape(map);
        values(map);
    }

    private void values(MapValue map) {
        for (MapValue.Entry entry : map.entries()) {
            value(entry.value());
        }
    }

    /** A shape's number if its keys were written before, else its keys (which makes it one). */
    private void shape(MapValue map) {
        List<Integer> numbers = map.entries().stream().map(e -> strings.get(e.key())).toList();
        Integer known = numbers.contains(null) ? null : shapes.get(numbers);
        if (known != null) {
            Varint.write(out, (long) known << 1);
            return;
        }
        Varint.write(out, ((long) map.size() << 1) | Binary.NEW_STRING);
        List<Integer> written = map.entries().stream().map(e -> string(e.key())).toList();
        shapes.put(written, shapes.size());
    }

    /** The string's bytes the first time, its table index after. */
    private int string(String text) {
        Integer index = strings.get(text);
        if (index != null) {
            Varint.write(out, (long) index << 1);
            return index;
        }
        byte[] raw = text.getBytes(StandardCharsets.UTF_8);
        Varint.write(out, ((long) raw.length << 1) | Binary.NEW_STRING);
        out.writeBytes(raw);
        int added = strings.size();
        strings.put(text, added);
        return added;
    }
}
