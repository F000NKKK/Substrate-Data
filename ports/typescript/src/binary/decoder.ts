import { isKindName, isRefId } from '../model/reference.js';
import { floatFromBits, nullValue, type MapEntry, type Value } from '../model/value.js';
import { widenBits } from '../model/float.js';
import { DecodeError } from './error.js';
import { MAGIC, MAX_DEPTH, NEW_STRING, STREAM_LEN_BYTES, Tag, VERSION } from './layout.js';
import { ByteReader } from './reader.js';
import { unzigzag } from './varint.js';

const F64_BYTES = 8;
const F32_BYTES = 4;
/** A map entry takes at least a key and a value's tag. */
const MAP_ENTRY_MIN = 2;

/** Reads values from bytes: the strings seen so far are the table. */
class ValueDecoder extends ByteReader {
  private readonly strings: string[] = [];

  constructor(
    bytes: Uint8Array,
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
        throw this.fail('unknown_tag', `unknown tag ${tag}`, tagAt);
    }
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
    const inner = new ValueDecoder(this.take(len), this.depth);
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
  const decoder = new ValueDecoder(bytes);
  const value = decoder.value();
  decoder.expectEnd();
  return value;
}

/**
 * Decodes a binary document (`SDAT`, version 1, one value).
 *
 * @throws DecodeError when the bytes are malformed; every count and length
 * is checked against the bytes left before anything is allocated.
 */
export function decodeBinary(bytes: Uint8Array): Value {
  if (bytes.length < MAGIC.length || MAGIC.some((b, i) => bytes[i] !== b)) {
    throw new DecodeError('bad_magic', 'not Substrate Data (no `SDAT` header)', 0);
  }
  const decoder = new ValueDecoder(bytes);
  decoder.pos = MAGIC.length;
  const version = decoder.byte();
  if (version !== VERSION) {
    throw new DecodeError(
      'bad_version',
      `Substrate Data version ${version}, this reads version ${VERSION}`,
      decoder.pos,
    );
  }
  const value = decoder.value();
  decoder.expectEnd();
  return value;
}

