/**
 * What a frame is read by: the description a schema gives of itself
 * (`Shape::describe`), as far as a frame's layout depends on it.
 */
export type Shape =
  | { readonly kind: 'bool' }
  | { readonly kind: 'int'; readonly min: bigint; readonly max: bigint }
  | { readonly kind: 'float'; readonly bits: 32 | 64 }
  | { readonly kind: 'string' }
  | { readonly kind: 'bytes' }
  | { readonly kind: 'optional'; readonly of: Shape }
  | { readonly kind: 'list'; readonly of: Shape }
  | { readonly kind: 'struct'; readonly fields: readonly Field[] }
  /** A unit enum (no `tag`): a variant index. With a `tag` it is held as its binary form. */
  | { readonly kind: 'enum'; readonly tag: string | undefined; readonly variants: readonly string[] }
  /** A reference of declared `kinds` packs only its id; of no kind, its kind and id as text. */
  | { readonly kind: 'ref'; readonly kinds: readonly string[]; readonly id: Shape }
  | { readonly kind: 'def'; readonly name: string }
  /** Anything else packs as its binary form. */
  | { readonly kind: 'binary' };

export interface Field {
  readonly name: string;
  readonly shape: Shape;
}

/** A root shape and the named shapes it uses. */
export interface ShapeDoc {
  readonly root: Shape;
  readonly defs: ReadonlyMap<string, Shape>;
}
