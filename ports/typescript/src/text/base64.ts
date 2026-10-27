const ALPHABET = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';

/** Standard base64, padded. */
export function base64Encode(bytes: Uint8Array): string {
  let out = '';
  for (let i = 0; i < bytes.length; i += 3) {
    const a = bytes[i] ?? 0;
    const b = bytes[i + 1] ?? 0;
    const c = bytes[i + 2] ?? 0;
    const word = (a << 16) | (b << 8) | c;
    out += ALPHABET[(word >> 18) & 63];
    out += ALPHABET[(word >> 12) & 63];
    out += i + 1 < bytes.length ? ALPHABET[(word >> 6) & 63] : '=';
    out += i + 2 < bytes.length ? ALPHABET[word & 63] : '=';
  }
  return out;
}
