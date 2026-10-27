/** Where a parsed value is in the text (UTF-16 indexes) and what is inside it. */
export interface Span {
  readonly start: number;
  readonly end: number;
  /** Where its key starts, for a map entry. */
  readonly keyStart?: number;
  /** Its items or entries, in order. */
  readonly children: readonly Span[];
}
