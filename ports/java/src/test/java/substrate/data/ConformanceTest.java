package substrate.data;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.stream.Stream;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.MethodSource;

/** Every case of {@code conformance/cases.json}, in every form it has. */
class ConformanceTest {
    private static final Path DIR = Path.of(System.getProperty("conformance.dir", "../../conformance"));

    static Stream<JsonNode> cases() throws IOException {
        JsonNode manifest = new ObjectMapper().readTree(Files.readAllBytes(DIR.resolve("cases.json")));
        List<JsonNode> cases = new ArrayList<>();
        manifest.forEach(cases::add);
        return cases.stream();
    }

    @ParameterizedTest(name = "{0}")
    @MethodSource("cases")
    void case_(JsonNode c) throws Exception {
        Value value = c.has("value") ? ExpectedValue.of(c.get("value")) : null;
        if (c.has("text")) {
            checkText(c, value);
        }
        if (c.has("binary")) {
            checkBinary(c, value);
        }
        if (c.has("frame")) {
            checkFrame(c.get("frame"));
        }
        if (c.has("io")) {
            checkIo(c.get("io"));
        }
        if (c.has("error")) {
            checkError(c.get("error"));
        }
    }

    private static void checkText(JsonNode c, Value value) throws Exception {
        if (value == null) {
            return;
        }
        String text = c.get("text").asText();
        assertEquals(Typed.flatten(value), Text.parse(text));
        if (c.get("text_is_written").asBoolean()) {
            assertEquals(text, Text.write(value));
        }
    }

    private static void checkBinary(JsonNode c, Value value) throws Exception {
        byte[] bytes = Files.readAllBytes(DIR.resolve(c.get("binary_file").asText()));
        assertEquals(c.get("binary").asText(), Hex.encode(bytes), "the manifest's hex is the file's bytes");
        Value decoded = Binary.decode(bytes);
        if (value != null) {
            assertEquals(value, decoded);
        }
        if (value == null || c.get("binary_is_encoded").asBoolean()) {
            assertArrayEquals(bytes, Binary.encode(value == null ? decoded : value));
        }
    }

    private static byte[] bytesOf(JsonNode node) throws IOException {
        if (node.has("bytes_file")) {
            return Files.readAllBytes(DIR.resolve(node.get("bytes_file").asText()));
        }
        return Hex.decode(node.get("bytes").asText());
    }

    private static void checkFrame(JsonNode frame) throws Exception {
        byte[] bytes = bytesOf(frame);
        assertEquals(ExpectedValue.of(frame.get("value")),
                Frames.read(bytes, ShapeDocument.parse(frame.get("shape").asText())));
        if (frame.has("also_read_by")) {
            for (JsonNode also : frame.get("also_read_by")) {
                assertEquals(ExpectedValue.of(also.get("value")),
                        Frames.read(bytes, ShapeDocument.parse(also.get("shape").asText())));
            }
        }
    }

    private static void checkIo(JsonNode io) throws Exception {
        ByteArrayInputStream in = new ByteArrayInputStream(bytesOf(io));
        for (JsonNode message : io.get("messages")) {
            assertArrayEquals(Hex.decode(message.asText()), FrameStream.read(in).orElseThrow());
        }
        assertFalse(FrameStream.read(in).isPresent(), "the stream ends cleanly");
    }

    private static void checkError(JsonNode error) throws Exception {
        switch (error.get("form").asText()) {
            case "text" -> checkTextError(error);
            case "text_schema" -> Text.parse(error.get("input").asText());
            case "binary" -> checkBinaryError(error);
            case "frame" -> assertThrows(DecodeException.class, () -> Frames.read(
                    Hex.decode(error.get("input").asText()), ShapeDocument.parse(error.get("shape").asText())));
            case "io" -> checkIoError(error);
            default -> throw new IllegalArgumentException("unknown error form " + error.get("form"));
        }
    }

    private static void checkTextError(JsonNode error) {
        TextSyntaxException e = assertThrows(TextSyntaxException.class, () -> Text.parse(error.get("input").asText()));
        assertEquals(error.get("line").asInt(), e.line(), e.getMessage());
        assertEquals(error.get("col").asInt(), e.column(), e.getMessage());
        assertEquals("SD0001", e.code());
    }

    private static void checkBinaryError(JsonNode error) {
        byte[] input = Hex.decode(error.get("input").asText());
        DecodeException e = assertThrows(DecodeException.class, () -> Binary.decode(input));
        assertEquals(error.get("kind").asText().toUpperCase(java.util.Locale.ROOT), e.kind().name(), e.getMessage());
        assertEquals(error.get("schema_code").asText(), e.code());
    }

    private static void checkIoError(JsonNode error) throws Exception {
        ByteArrayInputStream in = new ByteArrayInputStream(Hex.decode(error.get("input").asText()));
        for (int i = 0; i < error.get("frames_before").asInt(); i++) {
            FrameStream.read(in).orElseThrow();
        }
        FrameStreamException e = assertThrows(FrameStreamException.class, () -> FrameStream.read(in));
        assertEquals(error.get("kind").asText().toUpperCase(java.util.Locale.ROOT), e.kind().name());
    }
}
