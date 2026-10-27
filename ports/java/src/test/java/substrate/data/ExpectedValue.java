package substrate.data;

import com.fasterxml.jackson.databind.JsonNode;
import java.util.ArrayList;
import java.util.List;

/** A model value from the manifest's typed JSON ({@code {"t": "int", "v": "64"}}). */
final class ExpectedValue {
    private ExpectedValue() {}

    static Value of(JsonNode node) {
        String type = node.get("t").asText();
        return switch (type) {
            case "null" -> Null.INSTANCE;
            case "bool" -> new Bool(node.get("v").asBoolean());
            case "int" -> new Int(Long.parseLong(node.get("v").asText()));
            case "ints" -> ints(node.get("v"));
            case "float" -> Float.ofBits(Long.parseUnsignedLong(node.get("bits").asText(), 16));
            case "str" -> new Str(node.get("v").asText());
            case "list" -> list(node.get("v"));
            case "map" -> map(node.get("v"));
            case "ref" -> new Ref(node.get("kind").asText(), node.get("id").asText());
            case "bytes" -> new Bytes(Hex.decode(node.get("hex").asText()));
            default -> throw new IllegalArgumentException("unknown value type " + type);
        };
    }

    private static Value ints(JsonNode items) {
        List<Value> out = new ArrayList<>();
        items.forEach(n -> out.add(new Int(n.asLong())));
        return new ListValue(out);
    }

    private static Value list(JsonNode items) {
        List<Value> out = new ArrayList<>();
        items.forEach(n -> out.add(of(n)));
        return new ListValue(out);
    }

    private static Value map(JsonNode pairs) {
        List<MapValue.Entry> out = new ArrayList<>();
        pairs.forEach(p -> out.add(new MapValue.Entry(p.get(0).asText(), of(p.get(1)))));
        return new MapValue(out);
    }
}
