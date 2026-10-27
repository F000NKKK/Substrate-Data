import { lineColOf } from './line-col.js';
import { SyntaxError } from './syntax-error.js';

/** Deeper than this is an error, not a stack overflow. */
export const MAX_DEPTH = 256;

/** The text being read and the place in it. */
export class Cursor {
  pos = 0;
  depth = 0;

  constructor(readonly text: string) {}

  peek(): string | undefined {
    return this.text[this.pos];
  }

  startsWith(word: string): boolean {
    return this.text.startsWith(word, this.pos);
  }

  /** An error at the current place. */
  error(message: string): SyntaxError {
    return this.errorAt(this.pos, message);
  }

  errorAt(offset: number, message: string): SyntaxError {
    const suffix = offset >= this.text.length ? ', found the end of the file' : '';
    return new SyntaxError(message + suffix, lineColOf(this.text, offset), offset);
  }

  /** Skips whitespace and comments. */
  skipTrivia(): void {
    for (;;) {
      while (isSpace(this.peek())) {
        this.pos += 1;
      }
      if (this.peek() !== '/') {
        return;
      }
      const next = this.text[this.pos + 1];
      if (next === '/') {
        this.skipLineComment();
      } else if (next === '*') {
        this.skipBlockComment();
      } else {
        return;
      }
    }
  }

  private skipLineComment(): void {
    const end = this.text.indexOf('\n', this.pos);
    this.pos = end === -1 ? this.text.length : end;
  }

  private skipBlockComment(): void {
    const end = this.text.indexOf('*/', this.pos + 2);
    if (end === -1) {
      throw this.errorAt(this.pos, 'unclosed `/*` comment');
    }
    this.pos = end + 2;
  }
}

function isSpace(c: string | undefined): boolean {
  return c === ' ' || c === '\t' || c === '\n' || c === '\r';
}
