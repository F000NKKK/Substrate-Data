import type { LineCol } from './line-col.js';

/** Text that does not parse (code SD0001), and where. */
export class SyntaxError extends Error {
  constructor(
    message: string,
    readonly at: LineCol,
    readonly offset: number,
  ) {
    super(`${at.line}:${at.col}: ${message}`);
    this.name = 'SyntaxError';
  }
}
