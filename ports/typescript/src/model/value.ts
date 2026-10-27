// The model: one value type for every form.

import { bitsOf, numberOf } from './float.js';
import { InvalidValueError } from './errors.js';
import { isKindName, isRefId } from './reference.js';

export interface NullValue {
  readonly kind: 'null';
}
export interface BoolValue {
  readonly kind: 'bool';
  readonly value: boolean;
}
/** An `i64`. */
export interface IntValue {
  readonly kind: 'int';
  readonly value: bigint;
}
/** An `f64`, with its exact bits (`-0` and every NaN payload are kept). */
export interface FloatValue {
  readonly kind: 'float';
  readonly value: number;
  readonly bits: bigint;
}
/** A string as spelled: plain text starting with `@` is held with it doubled. */
export interface StrValue {
  readonly kind: 'str';
  readonly value: string;
}
export interface ListValue {
  readonly kind: 'list';
  readonly items: readonly Value[];
}
export type MapEntry = readonly [key: string, value: Value];
/** A map: keys are unique, and their order is part of the value. */
export interface MapValue {
  readonly kind: 'map';
  readonly entries: readonly MapEntry[];
}
export interface RefValue {
  readonly kind: 'ref';
  readonly refKind: string;
  readonly id: string;
}
export interface BytesValue {
  readonly kind: 'bytes';
  readonly value: Uint8Array;
}

export type Value =
  | NullValue
  | BoolValue
  | IntValue
  | FloatValue
  | StrValue
  | ListValue
  | MapValue
  | RefValue
  | BytesValue;

export const I64_MIN = -(1n << 63n);
export const I64_MAX = (1n << 63n) - 1n;

export const nullValue: NullValue = { kind: 'null' };

export function bool(value: boolean): BoolValue {
  return { kind: 'bool', value };
}

/** An int; a `number` must be a safe integer, a `bigint` must fit an `i64`. */
export function int(value: bigint | number): IntValue {
  if (typeof value === 'number' && !Number.isSafeInteger(value)) {
    throw new InvalidValueError(`${value} is not a safe integer; pass a bigint`);
  }
  const n = BigInt(value);
  if (n < I64_MIN || n > I64_MAX) {
    throw new InvalidValueError(`${n} does not fit an i64`);
  }
  return { kind: 'int', value: n };
}

export function float(value: number): FloatValue {
  return { kind: 'float', value, bits: bitsOf(value) };
}

export function floatFromBits(bits: bigint): FloatValue {
  const exact = BigInt.asUintN(64, bits);
  return { kind: 'float', value: numberOf(exact), bits: exact };
}

export function str(value: string): StrValue {
  return { kind: 'str', value };
}

export function list(items: readonly Value[]): ListValue {
  return { kind: 'list', items };
}

/** A map of `entries` in order; a repeated key is an error. */
export function map(entries: readonly MapEntry[]): MapValue {
  const seen = new Set<string>();
  for (const [key] of entries) {
    if (seen.has(key)) {
      throw new InvalidValueError(`\`${key}\` is already set in this map`);
    }
    seen.add(key);
  }
  return { kind: 'map', entries };
}

export function ref(refKind: string, id: string): RefValue {
  if (!isKindName(refKind)) {
    throw new InvalidValueError(`\`${refKind}\` isn't a kind`);
  }
  if (!isRefId(id)) {
    throw new InvalidValueError(`\`${id}\` isn't a reference's id`);
  }
  return { kind: 'ref', refKind, id };
}

export function bytes(value: Uint8Array): BytesValue {
  return { kind: 'bytes', value };
}
