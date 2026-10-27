package substrate.data;

import java.math.BigDecimal;
import java.math.MathContext;
import java.math.RoundingMode;

/**
 * A float in the shortest spelling that reads back as exactly it, and as a
 * float: {@code 1.0}, {@code 0.1}, {@code 1e-7}, {@code 1e16} (never
 * {@code 10000000000000000}, which would read back as an integer). Not finite
 * is written as {@code null}.
 */
final class FloatSpelling {
    private static final double EXPONENT_FROM = 1e16;
    private static final double EXPONENT_BELOW = 1e-4;

    private FloatSpelling() {}

    static String of(double value) {
        if (Double.isNaN(value) || Double.isInfinite(value)) {
            return "null";
        }
        String sign = (Double.doubleToRawLongBits(value) < 0) ? "-" : "";
        double magnitude = Math.abs(value);
        if (magnitude == 0.0) {
            return sign + "0.0";
        }
        BigDecimal digits = shortest(magnitude);
        boolean exponential = magnitude >= EXPONENT_FROM || magnitude < EXPONENT_BELOW;
        return sign + (exponential ? exponential(digits) : plain(digits));
    }

    /** The shortest decimal that reads back as {@code magnitude}, closest to it among those. */
    private static BigDecimal shortest(double magnitude) {
        BigDecimal exact = new BigDecimal(magnitude);
        for (int precision = 1; precision <= 17; precision++) {
            BigDecimal best = closestReadingBack(exact, magnitude, precision);
            if (best != null) {
                return best.stripTrailingZeros();
            }
        }
        return exact.round(new MathContext(17)).stripTrailingZeros();
    }

    private static BigDecimal closestReadingBack(BigDecimal exact, double magnitude, int precision) {
        BigDecimal down = exact.round(new MathContext(precision, RoundingMode.FLOOR));
        BigDecimal up = exact.round(new MathContext(precision, RoundingMode.CEILING));
        boolean downReads = down.doubleValue() == magnitude;
        boolean upReads = up.doubleValue() == magnitude;
        if (downReads && upReads) {
            return exact.subtract(down).compareTo(up.subtract(exact)) <= 0 ? down : up;
        }
        if (downReads) {
            return down;
        }
        return upReads ? up : null;
    }

    private static String exponential(BigDecimal digits) {
        String unscaled = digits.unscaledValue().toString();
        int exponent = unscaled.length() - 1 - digits.scale();
        String mantissa = unscaled.length() == 1 ? unscaled : unscaled.charAt(0) + "." + unscaled.substring(1);
        return mantissa + "e" + exponent;
    }

    private static String plain(BigDecimal digits) {
        String text = digits.toPlainString();
        return text.contains(".") ? text : text + ".0";
    }
}
