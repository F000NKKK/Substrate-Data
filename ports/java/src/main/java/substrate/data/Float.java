package substrate.data;

/** A float, compared by its IEEE 754 bits: {@code -0.0} is not {@code 0.0}, a NaN equals itself. */
public record Float(double value) implements Value {
    /** A float from its bits. */
    public static Float ofBits(long bits) {
        return new Float(Double.longBitsToDouble(bits));
    }

    /** The IEEE 754 bits. */
    public long bits() {
        return Double.doubleToRawLongBits(value);
    }

    @Override
    public boolean equals(Object other) {
        return other instanceof Float f && f.bits() == bits();
    }

    @Override
    public int hashCode() {
        return Long.hashCode(bits());
    }
}
