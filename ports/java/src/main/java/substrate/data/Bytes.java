package substrate.data;

import java.util.Arrays;

/** Bytes. The array is copied in and out, so the value stays immutable. */
public record Bytes(byte[] data) implements Value {
    public Bytes {
        data = data.clone();
    }

    @Override
    public byte[] data() {
        return data.clone();
    }

    @Override
    public boolean equals(Object other) {
        return other instanceof Bytes b && Arrays.equals(b.data, data);
    }

    @Override
    public int hashCode() {
        return Arrays.hashCode(data);
    }

    @Override
    public String toString() {
        return "Bytes" + Arrays.toString(data);
    }
}
