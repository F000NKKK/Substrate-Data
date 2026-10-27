import type { Value } from '../model/value.js';
import { ByteBuffer } from './byte-buffer.js';
import { MAGIC, NEW_STRING, Tag, VERSION } from './layout.js';
import { writeVarint, zigzag } from './varint.js';

const utf8 = new TextEncoder();

/** Writes the binary form: one buffer, each string written where it is first used. */
class BinaryWriter {
  private readonly out = new ByteBuffer();
  private readonly strings = new Map<string, number>();

  constructor() {
    this.out.pushAll(MAGIC);
    this.out.push(VERSION);
  }

  finish(): Uint8Array {
    return this.out.finish();
  }

  value(v: Value): void {
    switch (v.kind) {
      case 'null':
        this.out.push(Tag.Null);
        return;
      case 'bool':
        this.out.push(v.value ? Tag.True : Tag.False);
        return;
      case 'int':
        this.out.push(Tag.Int);
        writeVarint(this.out, zigzag(v.value));
        return;
      case 'float':
        this.float(v.bits);
        return;
      case 'str':
        this.out.push(Tag.Str);
        this.string(v.value);
        return;
      case 'list':
        this.list(v.items);
        return;
      case 'map':
        this.map(v.entries);
        return;
      case 'ref':
        this.out.push(Tag.Ref);
        this.string(v.refKind);
        this.string(v.id);
        return;
      case 'bytes':
        this.out.push(Tag.Bytes);
        writeVarint(this.out, v.value.length);
        this.out.pushAll(v.value);
        return;
    }
  }

  private float(bits: bigint): void {
    this.out.push(Tag.Float);
    for (let i = 0n; i < 8n; i += 1n) {
      this.out.push(Number((bits >> (8n * i)) & 0xffn));
    }
  }

  private list(items: readonly Value[]): void {
    this.out.push(Tag.List);
    writeVarint(this.out, items.length);
    for (const item of items) {
      this.value(item);
    }
  }

  private map(entries: readonly (readonly [string, Value])[]): void {
    this.out.push(Tag.Map);
    writeVarint(this.out, entries.length);
    for (const [key, item] of entries) {
      this.string(key);
      this.value(item);
    }
  }

  /** A string's index if it was written before, else its bytes (which add it to the table). */
  private string(s: string): void {
    const index = this.strings.get(s);
    if (index !== undefined) {
      writeVarint(this.out, BigInt(index) << 1n);
      return;
    }
    const encoded = utf8.encode(s);
    writeVarint(this.out, (BigInt(encoded.length) << 1n) | NEW_STRING);
    this.out.pushAll(encoded);
    this.strings.set(s, this.strings.size);
  }
}

/** Encodes `value` as a binary document, the bytes the reference writes. */
export function encodeBinary(value: Value): Uint8Array {
  const writer = new BinaryWriter();
  writer.value(value);
  return writer.finish();
}
