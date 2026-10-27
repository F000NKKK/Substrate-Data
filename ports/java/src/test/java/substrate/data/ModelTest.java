package substrate.data;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.util.List;
import java.util.Random;
import org.junit.jupiter.api.Test;

/** The model's own rules, beyond the vectors. */
class ModelTest {
    @Test
    void floatsCompareByBits() {
        assertNotEquals(new Float(0.0), new Float(-0.0));
        assertEquals(new Float(Double.NaN), new Float(Double.NaN));
    }

    @Test
    void mapsKeepOrderAndRejectDuplicateKeys() {
        MapValue.Entry a = new MapValue.Entry("a", new Int(1));
        MapValue.Entry b = new MapValue.Entry("b", new Int(2));
        assertNotEquals(MapValue.of(a, b), MapValue.of(b, a));
        assertThrows(IllegalArgumentException.class, () -> MapValue.of(a, a));
    }

    @Test
    void typedStringsAreClassified() throws Exception {
        assertEquals(new Ref("file", "a-b"), Typed.classify("@file:a-b"));
        assertEquals(new Str("@x"), Typed.classify("@@x"));
        assertEquals(new Bytes(new byte[] {1, 2, 3}), Typed.classify("@bytes:AQID"));
        for (String typo : List.of("@", "@logo", "@File:x", "@file:", "@file:a b", "@bytes:!!", "@bytes:AQI")) {
            assertThrows(TypedException.class, () -> Typed.classify(typo), typo);
        }
    }

    @Test
    void writtenFloatsReadBackAsTheSameFloat() throws Exception {
        Random random = new Random(7);
        for (int i = 0; i < 20_000; i++) {
            double d = Double.longBitsToDouble(random.nextLong());
            if (Double.isNaN(d) || Double.isInfinite(d)) {
                continue;
            }
            String text = Text.write(new Float(d));
            assertEquals(new Float(d), Text.parse(text), text);
        }
    }

    @Test
    void aWrittenFloatNeverReadsBackAsAnInteger() throws Exception {
        for (double d : new double[] {1e15, 1e16, 123456789012345680.0, 1.0, -3.0}) {
            assertEquals(new Float(d), Text.parse(Text.write(new Float(d))));
        }
    }
}
