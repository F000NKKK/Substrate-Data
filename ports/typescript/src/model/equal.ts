import type { Value } from './value.js';

/** Whether two values are the same: floats by their bits, maps with their order. */
export function valuesEqual(a: Value, b: Value): boolean {
  switch (a.kind) {
    case 'null':
      return b.kind === 'null';
    case 'bool':
    case 'int':
    case 'str':
      return a.kind === b.kind && a.value === b.value;
    case 'float':
      return b.kind === 'float' && a.bits === b.bits;
    case 'bytes':
      return b.kind === 'bytes' && sameBytes(a.value, b.value);
    case 'ref':
      return b.kind === 'ref' && a.refKind === b.refKind && a.id === b.id;
    case 'list':
      return b.kind === 'list' && sameItems(a.items, b.items);
    case 'map':
      return b.kind === 'map' && sameEntries(a, b);
  }
}

function sameBytes(a: Uint8Array, b: Uint8Array): boolean {
  return a.length === b.length && a.every((byte, i) => byte === b[i]);
}

function sameItems(a: readonly Value[], b: readonly Value[]): boolean {
  return a.length === b.length && a.every((item, i) => valuesEqual(item, b[i] as Value));
}

function sameEntries(
  a: { readonly entries: readonly (readonly [string, Value])[] },
  b: { readonly entries: readonly (readonly [string, Value])[] },
): boolean {
  return (
    a.entries.length === b.entries.length &&
    a.entries.every(([key, value], i) => {
      const other = b.entries[i] as readonly [string, Value];
      return key === other[0] && valuesEqual(value, other[1]);
    })
  );
}
