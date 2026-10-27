import type { MapEntry, Value } from '../model/value.js';
import { ByteBuffer } from './byte-buffer.js';
import { widenBits, narrowWord } from '../model/float.js';
import {
  INLINE_INT_MAX,
  INLINE_INT_MIN,
  INT_INLINE,
  MAGIC,
  NEW_STRING,
  STR_INDEX,
  STR_INDEX_COUNT,
  Tag,
  VERSION,
} from './layout.js';
import { writeVarint, zigzag } from './varint.js';

const utf8 = new TextEncoder();

type ListForm = 'items' | 'ints' | 'floats' | 'records';

/** How a list is written, chosen from the list alone: what every writer agrees on. */
function listForm(items: readonly Value[]): ListForm {
  const first = items[0];
  if (first === undefined) {
    return 'items';
  }
  if (first.kind === 'int' && items.every((item) => item.kind === 'int')) {
    return 'ints';
  }
  if (first.kind === 'float' && items.every((item) => item.kind === 'float')) {
    return 'floats';
  }
  if (first.kind === 'map' && first.entries.length > 0 && items.every((item) => sameKeys(first, item))) {
    return 'records';
  }
  return 'items';
}

function sameKeys(first: Value, other: Value): boolean {
  return (
    first.kind === 'map' &&
    other.kind === 'map' &&
    first.entries.length === other.entries.length &&
    first.entries.every(([key], i) => other.entries[i]?.[0] === key)
  );
}

const F64_EXPONENT_ALL = 0x7ffn;
const scratch = new DataView(new ArrayBuffer(8));

/** Whether the float with these bits is an `f32` exactly, bit for bit (a NaN never is). */
function narrows(bits: bigint): boolean {
  const exponent = (bits >> 52n) & F64_EXPONENT_ALL;
  if (exponent === F64_EXPONENT_ALL && (bits & ((1n << 52n) - 1n)) !== 0n) {
    return false;
  }
  return widenBits(narrowedWord(bits)) === bits;
}

/** The 32 bits of the float with these bits, rounded to an `f32`. */
function narrowedWord(bits: bigint): number {
  scratch.setBigUint64(0, bits);
  return narrowWord(scratch.getFloat64(0));
}

/** Writes the binary form: one buffer, each string written where it is first used. */
class BinaryWriter {
  private readonly out = new ByteBuffer();
  private readonly strings = new Map<string, number>();
  /** Shapes by their keys' string numbers. */
  private readonly shapes = new Map<string, number>();

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
        this.int(v.value);
        return;
      case 'float':
        this.float(v.bits);
        return;
      case 'str':
        this.str(v.value);
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

  /** An int: one byte for -16 to 111, else a tag and a varint. */
  private int(n: bigint): void {
    if (n >= BigInt(INLINE_INT_MIN) && n <= BigInt(INLINE_INT_MAX)) {
      this.out.push(INT_INLINE + Number(n - BigInt(INLINE_INT_MIN)));
      return;
    }
    this.out.push(Tag.Int);
    writeVarint(this.out, zigzag(n));
  }

  /** A float: four bytes when it is an `f32` exactly, else eight. */
  private float(bits: bigint): void {
    if (narrows(bits)) {
      this.out.push(Tag.Float32);
      this.littleEndian(narrowedWord(bits), 4);
      return;
    }
    this.out.push(Tag.Float);
    this.littleEndian(bits, 8);
  }

  private littleEndian(n: bigint | number, width: number): void {
    const bits = BigInt(n);
    for (let i = 0n; i < BigInt(width); i += 1n) {
      this.out.push(Number((bits >> (8n * i)) & 0xffn));
    }
  }

  /** A string value: a tag of its own when it is one of the first 32 of the table. */
  private str(s: string): void {
    const index = this.strings.get(s);
    if (index !== undefined && index < STR_INDEX_COUNT) {
      this.out.push(STR_INDEX + index);
      return;
    }
    this.out.push(Tag.Str);
    this.string(s);
  }

  /** A list: packed when every item is an int or a float, records when every item is a map of the first one's keys. */
  private list(items: readonly Value[]): void {
    const form = listForm(items);
    if (form === 'ints') {
      this.out.push(Tag.Ints);
      writeVarint(this.out, items.length);
      for (const item of items) {
        writeVarint(this.out, zigzag(item.kind === 'int' ? item.value : 0n));
      }
    } else if (form === 'floats') {
      this.floats(items);
    } else if (form === 'records') {
      this.records(items);
    } else {
      this.out.push(Tag.List);
      writeVarint(this.out, items.length);
      for (const item of items) {
        this.value(item);
      }
    }
  }

  private floats(items: readonly Value[]): void {
    const bits = items.map((item) => (item.kind === 'float' ? item.bits : 0n));
    const narrow = bits.every(narrows);
    this.out.push(narrow ? Tag.Floats32 : Tag.Floats);
    writeVarint(this.out, items.length);
    for (const b of bits) {
      if (narrow) {
        this.littleEndian(narrowedWord(b), 4);
      } else {
        this.littleEndian(b, 8);
      }
    }
  }

  private records(items: readonly Value[]): void {
    const first = items[0];
    this.out.push(Tag.Records);
    writeVarint(this.out, items.length);
    this.shape(first?.kind === 'map' ? first.entries : []);
    for (const item of items) {
      if (item.kind === 'map') {
        this.values(item.entries);
      }
    }
  }

  private map(entries: readonly MapEntry[]): void {
    if (entries.length === 0) {
      this.out.push(Tag.Map);
      writeVarint(this.out, 0);
      return;
    }
    this.out.push(Tag.MapShape);
    this.shape(entries);
    this.values(entries);
  }

  private values(entries: readonly MapEntry[]): void {
    for (const [, item] of entries) {
      this.value(item);
    }
  }

  /** A shape's number if its keys were written before, else its keys (which makes it one). */
  private shape(entries: readonly MapEntry[]): void {
    const numbers = entries.map(([key]) => this.strings.get(key));
    const id = numbers.join(',');
    const known = numbers.includes(undefined) ? undefined : this.shapes.get(id);
    if (known !== undefined) {
      writeVarint(this.out, BigInt(known) << 1n);
      return;
    }
    writeVarint(this.out, (BigInt(entries.length) << 1n) | NEW_STRING);
    const written = entries.map(([key]) => this.string(key));
    this.shapes.set(written.join(','), this.shapes.size);
  }

  /** A string's index if it was written before, else its bytes (which add it to the table). */
  private string(s: string): number {
    const index = this.strings.get(s);
    if (index !== undefined) {
      writeVarint(this.out, BigInt(index) << 1n);
      return index;
    }
    const encoded = utf8.encode(s);
    writeVarint(this.out, (BigInt(encoded.length) << 1n) | NEW_STRING);
    this.out.pushAll(encoded);
    const added = this.strings.size;
    this.strings.set(s, added);
    return added;
  }
}

/** Encodes `value` as a binary document, the bytes the reference writes. */
export function encodeBinary(value: Value): Uint8Array {
  const writer = new BinaryWriter();
  writer.value(value);
  return writer.finish();
}
