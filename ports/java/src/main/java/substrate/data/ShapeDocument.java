package substrate.data;

import java.util.Map;

/**
 * A shape with its named definitions: {@code { root, defs }}, the compact text
 * of {@code Shape::describe}.
 */
public record ShapeDocument(Shape root, Map<String, Shape> defs) {
    public ShapeDocument {
        defs = Map.copyOf(defs);
    }

    /**
     * Reads the description from its compact text.
     *
     * @throws IllegalArgumentException when the text is not a shape description
     */
    public static ShapeDocument parse(String text) {
        return ShapeParser.document(text);
    }
}
