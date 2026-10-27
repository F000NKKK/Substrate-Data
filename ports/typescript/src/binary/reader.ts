import { DecodeError, type DecodeErrorKind } from './error.js';
import { readVarint, type Position } from './varint.js';

const utf8 = new TextDecoder('utf-8', { fatal: true, ignoreBOM: true });

/** Bytes being read: every read checks the bytes left and says what is wrong. */
export class ByteReader implements Position {
  pos = 0;

  constructor(readonly bytes: Uint8Array) {}

  get left(): number {
    return this.bytes.length - this.pos;
  }

  get atEnd(): boolean {
    return this.pos === this.bytes.length;
  }

  fail(kind: DecodeErrorKind, message: string, offset = this.pos): DecodeError {
    return new DecodeError(kind, message, offset);
  }

  byte(): number {
    const b = this.bytes[this.pos];
    if (b === undefined) {
      throw this.fail('unexpected_end', 'unexpected end');
    }
    this.pos += 1;
    return b;
  }

  /** `n` bytes that must all be there (`unexpected end` otherwise). */
  fixed(n: number): DataView {
    if (n > this.left) {
      throw this.fail('unexpected_end', 'unexpected end');
    }
    const view = new DataView(this.bytes.buffer, this.bytes.byteOffset + this.pos, n);
    this.pos += n;
    return view;
  }

  varint(): bigint {
    const n = readVarint(this.bytes, this);
    if (n === undefined) {
      throw this.fail('bad_varint', 'a varint cut short or longer than 64 bits');
    }
    return n;
  }

  /** A count of things that each take at least `min` bytes: no more than fit in what is left. */
  count(min: number): number {
    const n = this.varint();
    const left = this.left;
    if (n > BigInt(Math.floor(left / Math.max(min, 1)))) {
      throw this.fail('bad_length', `length ${n} past the end (${left} bytes left)`);
    }
    return Number(n);
  }

  take(len: number): Uint8Array {
    if (len > this.left) {
      throw this.fail('bad_length', 'bytes past the end');
    }
    const out = this.bytes.subarray(this.pos, this.pos + len);
    this.pos += len;
    return out;
  }

  /** A varint length and that many bytes. */
  lengthBytes(): Uint8Array {
    const len = this.varint();
    if (len > BigInt(this.left)) {
      throw this.fail('bad_length', 'bytes past the end');
    }
    return this.take(Number(len));
  }

  utf8(bytes: Uint8Array, offset: number): string {
    try {
      return utf8.decode(bytes);
    } catch {
      throw this.fail('bad_utf8', 'a string isn\'t UTF-8', offset);
    }
  }
}
