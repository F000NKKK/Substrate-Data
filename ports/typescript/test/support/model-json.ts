// The manifest's tagged JSON values as model values.

import { bool, bytes, floatFromBits, int, list, map, nullValue, ref, str, typedSpelling, type Value } from '../../src/index.js';
import { fromHex } from './hex.js';
import type { JsonValue, ModelJson } from './manifest.js';

function items(v: JsonValue | undefined): JsonValue[] {
  if (!Array.isArray(v)) {
    throw new Error('expected an array');
  }
  return v;
}

function asModel(v: JsonValue | undefined): ModelJson {
  if (typeof v !== 'object' || v === null || Array.isArray(v)) {
    throw new Error('expected a tagged value');
  }
  return v as unknown as ModelJson;
}

function need<T>(v: T | undefined): T {
  if (v === undefined) {
    throw new Error('a tagged value misses a field');
  }
  return v;
}

export function modelValue(json: ModelJson): Value {
  switch (json.t) {
    case 'null':
      return nullValue;
    case 'bool':
      return bool(json.v === true);
    case 'int':
      return int(BigInt(need(json.v) as string));
    case 'ints':
      return list(items(json.v).map((n) => int(BigInt(n as number))));
    case 'float':
      return floatFromBits(BigInt(`0x${need(json.bits)}`));
    case 'str':
      return str(need(json.v) as string);
    case 'list':
      return list(items(json.v).map((v) => modelValue(asModel(v))));
    case 'map':
      return map(items(json.v).map((pair) => pairOf(items(pair))));
    case 'ref':
      return ref(need(json.kind), need(json.id));
    case 'bytes':
      return bytes(fromHex(need(json.hex)));
    default:
      throw new Error(`unknown value tag ${json.t}`);
  }
}

function pairOf(pair: JsonValue[]): [string, Value] {
  return [pair[0] as string, modelValue(asModel(pair[1]))];
}

/** The value a text parse gives: refs and bytes as the strings they are written as. */
export function textModel(value: Value): Value {
  if (value.kind === 'ref' || value.kind === 'bytes') {
    return str(typedSpelling(value) ?? '');
  }
  if (value.kind === 'list') {
    return list(value.items.map(textModel));
  }
  if (value.kind === 'map') {
    return map(value.entries.map(([k, v]) => [k, textModel(v)]));
  }
  return value;
}
