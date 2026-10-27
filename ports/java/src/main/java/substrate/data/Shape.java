package substrate.data;

import java.util.List;

/** The description of a value a frame is packed from: what a reader needs to unpack it. */
public sealed interface Shape {
    /** A bool. */
    record BoolShape() implements Shape {}

    /** An integer in {@code min..=max}. */
    record IntShape(long min, long max) implements Shape {}

    /** A float of 32 or 64 bits. */
    record FloatShape(int bits) implements Shape {}

    /** A string. */
    record StringShape() implements Shape {}

    /** Bytes. */
    record BytesShape() implements Shape {}

    /** A value that may be absent. */
    record OptionalShape(Shape of) implements Shape {}

    /** A list. */
    record ListShape(Shape of) implements Shape {}

    /** A reference: of {@code kinds} (none: any kind), its id packed as {@code id} says. */
    record RefShape(List<String> kinds, Shape id) implements Shape {
        public RefShape {
            kinds = List.copyOf(kinds);
        }
    }

    /** A record: fields in declaration order. */
    record StructShape(List<Field> fields) implements Shape {
        public StructShape {
            fields = List.copyOf(fields);
        }
    }

    /** One field of a record. */
    record Field(String name, Shape shape, boolean required) {}

    /** An enum: of names (a variant index) when untagged, else held in its binary form. */
    record EnumShape(boolean tagged, List<String> variants) implements Shape {
        public EnumShape {
            variants = List.copyOf(variants);
        }
    }

    /** A named definition of the document's {@code defs}. */
    record DefShape(String name) implements Shape {}

    /** Anything else: held in its binary form. */
    record OtherShape(String kind) implements Shape {}
}
