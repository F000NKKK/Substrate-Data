package substrate.data;

import java.io.ByteArrayOutputStream;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.util.HashMap;
import java.util.Map;

/** Writes a {@link Value} in the binary form: strings interned, written where first used. */
final class BinaryEncoder {
    private final ByteArrayOutputStream out = new ByteArrayOutputStream();
    private final Map<String, Integer> strings = new HashMap<>();

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
            case Int i -> {
                out.write(Tag.INT);
                Varint.write(out, Varint.zigzag(i.value()));
            }
            case Float f -> {
                out.write(Tag.FLOAT);
                out.writeBytes(ByteBuffer.allocate(Double.BYTES).order(ByteOrder.LITTLE_ENDIAN).putLong(f.bits()).array());
            }
            case Str s -> {
                out.write(Tag.STR);
                string(s.value());
            }
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

    private void list(ListValue list) {
        out.write(Tag.LIST);
        Varint.write(out, list.items().size());
        list.items().forEach(this::value);
    }

    private void map(MapValue map) {
        out.write(Tag.MAP);
        Varint.write(out, map.size());
        for (MapValue.Entry entry : map.entries()) {
            string(entry.key());
            value(entry.value());
        }
    }

    /** The string's bytes the first time, its table index after. */
    private void string(String text) {
        Integer index = strings.get(text);
        if (index != null) {
            Varint.write(out, (long) index << 1);
            return;
        }
        byte[] raw = text.getBytes(StandardCharsets.UTF_8);
        Varint.write(out, ((long) raw.length << 1) | Binary.NEW_STRING);
        out.writeBytes(raw);
        strings.put(text, strings.size());
    }
}
