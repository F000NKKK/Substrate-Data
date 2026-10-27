import { isKindName, isRefId } from '../model/reference.js';
import { floatFromBits, nullValue, type MapEntry, type Value } from '../model/value.js';
import { widenBits } from '../model/float.js';
import { DecodeError } from './error.js';
import {
  INLINE_INT_MIN,
  INT_INLINE,
  LEGACY_VERSION,
  MAGIC,
  MAX_DEPTH,
  NEW_STRING,
  STREAM_LEN_BYTES,
  STR_INDEX,
  STR_INDEX_COUNT,
  Tag,
  VERSION,
} from './layout.js';
import { ByteReader } from './reader.js';
import { unzigzag } from './varint.js';

const F64_BYTES = 8;
const F32_BYTES = 4;
/** A map entry takes at least a key and a value's tag. */
const MAP_ENTRY_MIN = 2;

/** Reads values from bytes: the strings seen so far are the table. */
class ValueDecoder extends ByteReader {
  private readonly strings: string[] = [];
  /** The map shapes read so far (version 2): each its keys. */
  private readonly shapes: string[][] = [];

  constructor(
    bytes: Uint8Array,
    private readonly v2: boolean,
    private depth = 0,
  ) {
    super(bytes);
  }

  value(): Value {
    const tagAt = this.pos;
    const tag = this.byte();
    switch (tag) {
      case Tag.Null:
        return nullValue;
      case Tag.False:
        return { kind: 'bool', value: false };
      case Tag.True:
        return { kind: 'bool', value: true };
      case Tag.Int:
        return { kind: 'int', value: unzigzag(this.varint()) };
      case Tag.Float:
        return floatFromBits(this.fixed(F64_BYTES).getBigUint64(0, true));
      case Tag.Float32:
        return floatFromBits(widenBits(this.fixed(F32_BYTES).getUint32(0, true)));
      case Tag.Str:
        return { kind: 'str', value: this.string() };
      case Tag.List:
        return this.listValue();
      case Tag.Map:
        return this.mapValue();
      case Tag.Ref:
        return this.refValue();
      case Tag.Bytes:
        return { kind: 'bytes', value: this.lengthBytes().slice() };
      case Tag.Ints:
        return this.intsValue();
      case Tag.Stream:
        return this.streamValue();
      case Tag.Floats:
        return this.floatsValue(F64_BYTES);
      case Tag.Floats32:
        return this.floatsValue(F32_BYTES);
      default:
        return this.version2(tag, tagAt);
    }
  }

  /** A tag only version 2 has: shapes, records, an int or a string by itself. */
  private version2(tag: number, tagAt: number): Value {
    if (this.v2) {
      if (tag === Tag.MapShape) {
        return this.shapedMap(this.shapeSpec());
      }
      if (tag === Tag.Records) {
        return this.recordsValue();
      }
      if (tag >= INT_INLINE) {
        return { kind: 'int', value: BigInt(tag - INT_INLINE + INLINE_INT_MIN) };
      }
      if (tag >= STR_INDEX && tag < STR_INDEX + STR_INDEX_COUNT) {
        return { kind: 'str', value: this.known(BigInt(tag - STR_INDEX)) };
      }
    }
    throw this.fail('unknown_tag', `unknown tag ${tag}`, tagAt);
  }

  /** A shape: its number, or its keys (the first time): its keys. */
  private shapeSpec(): string[] {
    const marker = this.varint();
    if ((marker & NEW_STRING) === 0n) {
      const shape = marker >> 1n;
      const keys = shape < BigInt(this.shapes.length) ? this.shapes[Number(shape)] : undefined;
      if (keys === undefined) {
        throw this.fail('bad_shape_index', `shape ${shape} isn't defined`);
      }
      return keys;
    }
    const n = marker >> 1n;
    if (n === 0n) {
      throw this.fail('bad_length', 'a shape with no keys');
    }
    if (n > BigInt(this.left)) {
      throw this.fail('bad_length', `length ${n} past the end (${this.left} bytes left)`);
    }
    const keys: string[] = [];
    const seen = new Set<string>();
    for (let i = 0n; i < n; i += 1n) {
      const at = this.pos;
      const key = this.string();
      if (seen.has(key)) {
        throw this.fail('duplicate_key', `\`${key}\` is already set in this map`, at);
      }
      seen.add(key);
      keys.push(key);
    }
    this.shapes.push(keys);
    return keys;
  }

  private shapedMap(keys: readonly string[]): Value {
    if (keys.length > this.left) {
      throw this.fail('bad_length', `length ${keys.length} past the end (${this.left} bytes left)`);
    }
    this.enter();
    const map = this.shapedEntries(keys);
    this.depth -= 1;
    return map;
  }

  private shapedEntries(keys: readonly string[]): Value {
    const entries: MapEntry[] = [];
    for (const key of keys) {
      entries.push([key, this.value()]);
    }
    return { kind: 'map', entries };
  }

  /** A list of maps of one shape: a count, the shape, the values. */
  private recordsValue(): Value {
    const n = this.count(1);
    const keys = this.shapeSpec();
    if (n > Math.floor(this.left / keys.length)) {
      throw this.fail('bad_length', `length ${n} past the end (${this.left} bytes left)`);
    }
    this.enter();
    const items: Value[] = [];
    for (let i = 0; i < n; i += 1) {
      this.enter();
      items.push(this.shapedEntries(keys));
      this.depth -= 1;
    }
    this.depth -= 1;
    return { kind: 'list', items };
  }

  private string(): string {
    const marker = this.varint();
    if ((marker & NEW_STRING) === 0n) {
      return this.known(marker >> 1n);
    }
    const len = marker >> 1n;
    if (len > BigInt(this.left)) {
      throw this.fail('bad_length', 'bytes past the end');
    }
    const at = this.pos;
    const text = this.utf8(this.take(Number(len)), at);
    this.strings.push(text);
    return text;
  }

  private known(index: bigint): string {
    const text = index < BigInt(this.strings.length) ? this.strings[Number(index)] : undefined;
    if (text === undefined) {
      throw this.fail('bad_string_index', `string ${index} isn't in the table`);
    }
    return text;
  }

  private enter(): void {
    this.depth += 1;
    if (this.depth > MAX_DEPTH) {
      throw this.fail('too_deep', `nested deeper than ${MAX_DEPTH} levels`);
    }
  }

  private listValue(): Value {
    const n = this.count(1);
    this.enter();
    const items: Value[] = [];
    for (let i = 0; i < n; i += 1) {
      items.push(this.value());
    }
    this.depth -= 1;
    return { kind: 'list', items };
  }

  private mapValue(): Value {
    const n = this.count(MAP_ENTRY_MIN);
    this.enter();
    const entries: MapEntry[] = [];
    const seen = new Set<string>();
    for (let i = 0; i < n; i += 1) {
      const at = this.pos;
      const key = this.string();
      if (seen.has(key)) {
        throw this.fail('duplicate_key', `\`${key}\` is already set in this map`, at);
      }
      seen.add(key);
      entries.push([key, this.value()]);
    }
    this.depth -= 1;
    return { kind: 'map', entries };
  }

  private refValue(): Value {
    const kindAt = this.pos;
    const refKind = this.string();
    if (!isKindName(refKind)) {
      throw this.fail('bad_reference', `\`${refKind}\` isn't a kind`, kindAt);
    }
    const idAt = this.pos;
    const id = this.string();
    if (!isRefId(id)) {
      throw this.fail('bad_reference', `\`${id}\` isn't a reference's id`, idAt);
    }
    return { kind: 'ref', refKind, id };
  }

  private intsValue(): Value {
    const n = this.count(1);
    const items: Value[] = [];
    for (let i = 0; i < n; i += 1) {
      items.push({ kind: 'int', value: unzigzag(this.varint()) });
    }
    return { kind: 'list', items };
  }

  private floatsValue(width: number): Value {
    const n = this.count(width);
    const view = this.fixed(n * width);
    const items: Value[] = [];
    for (let i = 0; i < n; i += 1) {
      items.push(
        width === F64_BYTES
          ? floatFromBits(view.getBigUint64(i * width, true))
          : floatFromBits(widenBits(view.getUint32(i * width, true))),
      );
    }
    return { kind: 'list', items };
  }

  /** A stream: its own string table, its offsets relative to its body. */
  private streamValue(): Value {
    const len = this.fixed(STREAM_LEN_BYTES).getUint32(0, true);
    const inner = new ValueDecoder(this.take(len), this.v2, this.depth);
    const n = inner.count(1);
    const items: Value[] = [];
    for (let i = 0; i < n; i += 1) {
      items.push(inner.value());
    }
    inner.expectEnd();
    return { kind: 'list', items };
  }

  expectEnd(): void {
    if (!this.atEnd) {
      throw this.fail('trailing_bytes', 'bytes after the value');
    }
  }
}

/** Reads one value from a binary value body (no header): the form a frame holds a tagged enum in. */
export function decodeBody(bytes: Uint8Array): Value {
  const decoder = new ValueDecoder(bytes, true);
  const value = decoder.value();
  decoder.expectEnd();
  return value;
}

/**
 * Decodes a binary document (`SDAT`, version 2 or the older 1, one value).
 *
 * @throws DecodeError when the bytes are malformed; every count and length
 * is checked against the bytes left before anything is allocated.
 */
export function decodeBinary(bytes: Uint8Array): Value {
  if (bytes.length < MAGIC.length || MAGIC.some((b, i) => bytes[i] !== b)) {
    throw new DecodeError('bad_magic', 'not Substrate Data (no `SDAT` header)', 0);
  }
  const version = bytes[MAGIC.length];
  const probe = new ByteReader(bytes);
  probe.pos = MAGIC.length;
  probe.byte();
  if (version !== VERSION && version !== LEGACY_VERSION) {
    throw new DecodeError(
      'bad_version',
      `Substrate Data version ${version}, this reads versions ${LEGACY_VERSION} and ${VERSION}`,
      probe.pos,
    );
  }
  const decoder = new ValueDecoder(bytes, version === VERSION);
  decoder.pos = probe.pos;
  const value = decoder.value();
  decoder.expectEnd();
  return value;
}

