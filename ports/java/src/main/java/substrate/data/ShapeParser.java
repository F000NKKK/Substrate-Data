package substrate.data;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/** A shape description (a text value) into {@link Shape}s. */
final class ShapeParser {
    private ShapeParser() {}

    static ShapeDocument document(String text) {
        Value parsed;
        try {
            parsed = Text.parse(text);
        } catch (TextSyntaxException e) {
            throw new IllegalArgumentException("a shape description that doesn't parse: " + e.getMessage(), e);
        }
        MapValue map = asMap(parsed);
        Map<String, Shape> defs = new HashMap<>();
        for (MapValue.Entry def : asMap(field(map, "defs")).entries()) {
            defs.put(def.key(), shape(def.value()));
        }
        return new ShapeDocument(shape(field(map, "root")), defs);
    }

    private static Shape shape(Value value) {
        MapValue map = asMap(value);
        String kind = string(field(map, "kind"));
        return switch (kind) {
            case "bool" -> new Shape.BoolShape();
            case "int" -> new Shape.IntShape(number(map, "min", Long.MIN_VALUE), number(map, "max", Long.MAX_VALUE));
            case "float" -> new Shape.FloatShape((int) number(map, "bits", 64));
            case "string" -> new Shape.StringShape();
            case "bytes" -> new Shape.BytesShape();
            case "optional" -> new Shape.OptionalShape(shape(field(map, "of")));
            case "list" -> new Shape.ListShape(shape(field(map, "of")));
            case "ref" -> new Shape.RefShape(strings(field(map, "of")), shape(field(map, "id")));
            case "def" -> new Shape.DefShape(string(field(map, "name")));
            case "struct" -> struct(map);
            case "enum" -> new Shape.EnumShape(map.get("tag").isPresent(), variantNames(field(map, "variants")));
            default -> new Shape.OtherShape(kind);
        };
    }

    private static Shape struct(MapValue map) {
        List<Shape.Field> fields = new ArrayList<>();
        for (Value item : asList(field(map, "fields")).items()) {
            MapValue f = asMap(item);
            boolean required = f.get("required").map(v -> v instanceof Bool b && b.value()).orElse(true);
            fields.add(new Shape.Field(string(field(f, "name")), shape(field(f, "shape")), required));
        }
        return new Shape.StructShape(fields);
    }

    private static List<String> variantNames(Value variants) {
        return asList(variants).items().stream().map(v -> string(field(asMap(v), "name"))).toList();
    }

    private static List<String> strings(Value value) {
        return asList(value).items().stream().map(ShapeParser::string).toList();
    }

    private static long number(MapValue map, String key, long fallback) {
        return map.get(key).map(ShapeParser::integer).orElse(fallback);
    }

    private static long integer(Value value) {
        if (value instanceof Int i) {
            return i.value();
        } else if (value instanceof Float f) {
            // Past i64 (a u64 bound): the largest an Int holds.
            return f.value() > 0 ? Long.MAX_VALUE : Long.MIN_VALUE;
        }
        throw new IllegalArgumentException("a number was expected in a shape");
    }

    private static Value field(MapValue map, String key) {
        return map.get(key).orElseThrow(() -> new IllegalArgumentException("a shape lacks `" + key + "`"));
    }

    private static MapValue asMap(Value value) {
        if (value instanceof MapValue m) {
            return m;
        }
        throw new IllegalArgumentException("an object was expected in a shape");
    }

    private static ListValue asList(Value value) {
        if (value instanceof ListValue l) {
            return l;
        }
        throw new IllegalArgumentException("a list was expected in a shape");
    }

    private static String string(Value value) {
        if (value instanceof Str s) {
            return s.value();
        }
        throw new IllegalArgumentException("a string was expected in a shape");
    }
}
