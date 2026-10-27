import { decodeBody } from '../binary/decoder.js';
import { DecodeError } from '../binary/error.js';
import { unzigzag } from '../binary/varint.js';
import { isKindName, isRefId } from '../model/reference.js';
import { floatFromBits, I64_MAX, nullValue, type MapEntry, type Value } from '../model/value.js';
import { FrameError } from './error.js';
import { unpackLanes } from './lanes.js';
import type { Shape, ShapeDoc } from './shape.js';
import { Unpacker } from './unpacker.js';

/** The largest `u64` a 64-bit unsigned list may hold, as an `i64`'s bits. */
const I64_BITS = 63n;

/**
 * Reads a frame by its shape alone, as a value: what packs itself is read
 * as packed, everything else as its binary form. Plain text comes out as the
 * model spells it (a leading `@` doubled).
 *
 * @throws FrameError when `bytes` is not a frame of that shape.
 */
export function readFrame(bytes: Uint8Array, shape: ShapeDoc): Value {
  const unpacker = new Unpacker(bytes);
  const value = new ShapedReader(unpacker, shape).value(shape.root);
  if (!unpacker.atEnd) {
    throw new FrameError('bytes left in the frame after its value');
  }
  return value;
}

class ShapedReader {
  constructor(
    private readonly u: Unpacker,
    private readonly doc: ShapeDoc,
  ) {}

  value(shape: Shape): Value {
    const s = this.resolve(shape);
    switch (s.kind) {
      case 'bool':
        return { kind: 'bool', value: this.u.bool() };
      case 'int':
        return { kind: 'int', value: this.u.int() };
      case 'float':
        return floatFromBits(s.bits === 32 ? this.u.f32Bits() : this.u.f64Bits());
      case 'string':
        return { kind: 'str', value: escapeAt(this.u.str()) };
      case 'bytes':
        return { kind: 'bytes', value: this.u.bytes().slice() };
      case 'optional':
        return this.u.bool() ? this.value(s.of) : nullValue;
      case 'list':
        return this.list(this.resolve(s.of));
      case 'struct':
        return this.record(s.fields);
      case 'ref':
        return this.reference(s.kinds, s.id);
      case 'enum':
        return s.tag === undefined ? this.variantName(s.variants) : this.binary();
      case 'binary':
        return this.binary();
      case 'def':
        throw new FrameError(`\`${s.name}\` is not defined`);
    }
  }

  /** The shape a `def` names, followed through other defs. */
  private resolve(shape: Shape): Shape {
    let current = shape;
    for (let hops = 0; current.kind === 'def'; hops += 1) {
      const next = this.doc.defs.get(current.name);
      if (next === undefined || hops > this.doc.defs.size) {
        throw new FrameError(`\`${current.name}\` is not defined`);
      }
      current = next;
    }
    return current;
  }

  /** A value held in its binary form: own string table, no header. */
  private binary(): Value {
    try {
      return decodeBody(this.u.bytes());
    } catch (e) {
      if (e instanceof DecodeError) {
        throw new FrameError(`a value held in its binary form: ${e.message}`);
      }
      throw e;
    }
  }

  private variantName(variants: readonly string[]): Value {
    const index = this.u.uint();
    const name = index < BigInt(variants.length) ? variants[Number(index)] : undefined;
    if (name === undefined) {
      throw new FrameError(`variant ${index} is not one of ${variants.length}`);
    }
    return { kind: 'str', value: name };
  }

  /** A record: its fields in order by name; fields the shape does not know skipped, missing ones left out. */
  private record(fields: readonly { name: string; shape: Shape }[]): Value {
    const end = this.u.begin();
    const entries: MapEntry[] = [];
    for (const field of fields) {
      if (!this.u.more(end)) {
        break;
      }
      entries.push([field.name, this.value(field.shape)]);
    }
    this.u.leaveLevel();
    this.u.skipTo(end);
    return { kind: 'map', entries };
  }

  /** A list: numbers packed, anything else item by item. */
  private list(item: Shape): Value {
    if (item.kind === 'int') {
      return this.ints(item.min, item.max);
    }
    if (item.kind === 'float') {
      const bits = this.u.packedFloats(item.bits === 64);
      return { kind: 'list', items: bits.map(floatFromBits) };
    }
    const { n, end } = this.u.list();
    this.u.enter();
    const items: Value[] = [];
    for (let i = 0; i < n; i += 1) {
      items.push(this.value(item));
    }
    this.u.leaveLevel();
    this.u.skipTo(end);
    return { kind: 'list', items };
  }

  private ints(min: bigint, max: bigint): Value {
    const { n, width, raw } = this.u.packedInts();
    const numbers = unpackLanes(raw, n, width);
    const toInt = intReader(min, max);
    return { kind: 'list', items: numbers.map((v) => ({ kind: 'int', value: toInt(v) })) };
  }

  /** A reference of declared kinds packs only its id; of any kind, its kind and id as text. */
  private reference(kinds: readonly string[], id: Shape): Value {
    const refKind = kinds[0] ?? this.u.str();
    const spelled = kinds.length > 0 ? this.idText(id) : this.u.str();
    if (!isKindName(refKind) || !isRefId(spelled)) {
      throw new FrameError(`\`${refKind}:${spelled}\` is not a reference`);
    }
    return { kind: 'ref', refKind, id: spelled };
  }

  private idText(id: Shape): string {
    return this.resolve(id).kind === 'int' ? this.u.int().toString() : this.u.str();
  }
}

/** How a packed number becomes an int: zigzagged when the list may hold negatives, else its bits. */
function intReader(min: bigint, max: bigint): (v: bigint) => bigint {
  if (min < 0n) {
    return unzigzag;
  }
  if (max >= I64_MAX) {
    return (v) => BigInt.asIntN(64, v);
  }
  return (v) => {
    if (v >> I64_BITS !== 0n) {
      throw new FrameError('a number past what an i64 holds in a list of narrower ints');
    }
    return v;
  };
}

function escapeAt(s: string): string {
  return s.startsWith('@') ? `@${s}` : s;
}
