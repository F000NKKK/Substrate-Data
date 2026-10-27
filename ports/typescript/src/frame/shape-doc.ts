import { parseText } from '../text/parser.js';
import type { MapValue, Value } from '../model/value.js';
import { FrameError } from './error.js';
import type { Field, Shape, ShapeDoc } from './shape.js';

/** Reads a shape description (`{ root, defs }`, as compact text) a schema gave of itself. */
export function parseShapeDoc(text: string): ShapeDoc {
  const doc = asMap(parseText(text), 'the shape description');
  const defs = new Map<string, Shape>();
  for (const [name, shape] of asMap(member(doc, 'defs'), 'defs').entries) {
    defs.set(name, shapeOf(shape));
  }
  return { root: shapeOf(member(doc, 'root')), defs };
}

function asMap(value: Value | undefined, what: string): MapValue {
  if (value?.kind !== 'map') {
    throw new FrameError(`${what} is not an object`);
  }
  return value;
}

function member(map: MapValue, key: string): Value | undefined {
  return map.entries.find(([k]) => k === key)?.[1];
}

function text(map: MapValue, key: string): string {
  const v = member(map, key);
  if (v?.kind !== 'str') {
    throw new FrameError(`a shape needs a text \`${key}\``);
  }
  return v.value;
}

function shapeOf(value: Value | undefined): Shape {
  const map = asMap(value, 'a shape');
  const kind = text(map, 'kind');
  switch (kind) {
    case 'bool':
    case 'string':
    case 'bytes':
      return { kind };
    case 'int':
      return { kind, min: bound(map, 'min'), max: bound(map, 'max') };
    case 'float':
      return { kind, bits: bound(map, 'bits') === 32n ? 32 : 64 };
    case 'optional':
    case 'list':
      return { kind, of: shapeOf(member(map, 'of')) };
    case 'struct':
      return { kind, fields: fieldsOf(member(map, 'fields')) };
    case 'enum':
      return enumOf(map);
    case 'ref':
      return refOf(map);
    case 'def':
      return { kind, name: text(map, 'name') };
    case 'named':
      return shapeOf(member(map, 'of'));
    case 'atom':
      throw new FrameError('an atom is a number in a process\'s table: it has no meaning outside it');
    default:
      return { kind: 'binary' };
  }
}

function bound(map: MapValue, key: string): bigint {
  const v = member(map, key);
  if (v?.kind !== 'int') {
    throw new FrameError(`a shape needs an integer \`${key}\``);
  }
  return v.value;
}

function fieldsOf(value: Value | undefined): Field[] {
  if (value?.kind !== 'list') {
    throw new FrameError('a struct needs its fields');
  }
  return value.items.map((item) => {
    const field = asMap(item, 'a field');
    return { name: text(field, 'name'), shape: shapeOf(member(field, 'shape')) };
  });
}

function enumOf(map: MapValue): Shape {
  const variants = member(map, 'variants');
  if (variants?.kind !== 'list') {
    throw new FrameError('an enum needs its variants');
  }
  const tag = member(map, 'tag');
  return {
    kind: 'enum',
    tag: tag?.kind === 'str' ? tag.value : undefined,
    variants: variants.items.map((v) => text(asMap(v, 'a variant'), 'name')),
  };
}

function refOf(map: MapValue): Shape {
  const kinds = member(map, 'of');
  if (kinds?.kind !== 'list') {
    throw new FrameError('a reference needs its kinds');
  }
  return {
    kind: 'ref',
    kinds: kinds.items.map((k) => (k.kind === 'str' ? k.value : '')),
    id: shapeOf(member(map, 'id')),
  };
}
