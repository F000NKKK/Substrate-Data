package substrate.data;

import java.util.HexFormat;

/** Lower case hex without separators, as the manifest writes bytes. */
final class Hex {
    private Hex() {}

    static byte[] decode(String hex) {
        return HexFormat.of().parseHex(hex);
    }

    static String encode(byte[] bytes) {
        return HexFormat.of().formatHex(bytes);
    }
}
