/** A 1-based line and column; the column counts characters (code points). */
export interface LineCol {
  readonly line: number;
  readonly col: number;
}

/** Where UTF-16 index `offset` of `text` is. */
export function lineColOf(text: string, offset: number): LineCol {
  const end = Math.min(offset, text.length);
  let line = 1;
  let lineStart = 0;
  for (let i = 0; i < end; i += 1) {
    if (text.charCodeAt(i) === 0x0a) {
      line += 1;
      lineStart = i + 1;
    }
  }
  let col = 1;
  for (let i = lineStart; i < end; i += 1) {
    const unit = text.charCodeAt(i);
    const isLowSurrogate = unit >= 0xdc00 && unit < 0xe000;
    if (!(isLowSurrogate && i > lineStart)) {
      col += 1;
    }
  }
  return { line, col };
}
