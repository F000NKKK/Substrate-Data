import { MAX_DEPTH } from '../binary/layout.js';
import { readVarint, unzigzag, type Position } from '../binary/varint.js';
import { widenBits } from '../model/float.js';
import { FrameError } from './error.js';
import { packedBytes } from './lanes.js';

const U64_BITS = 64;
const F64_BYTES = 8;
const F32_BYTES = 4;
const utf8 = new TextDecoder('utf-8', { fatal: true, ignoreBOM: true });

/** A varint written backwards at the end of `bytes`: its value and how many bytes it took. */
function readVarintBack(bytes: Uint8Array): { value: number; taken: number } {
  let n = 0n;
  let shift = 0n;
  for (let taken = 1; taken <= bytes.length; taken += 1) {
    const byte = bytes[bytes.length - taken] as number;
    const bits = BigInt(byte & 0x7f);
    if (shift >= BigInt(U64_BITS) || (shift === 63n && bits > 1n)) {
      break;
    }
    n |= bits << shift;
    if ((byte & 0x80) === 0) {
      if (n > BigInt(Number.MAX_SAFE_INTEGER)) {
        break;
      }
      return { value: Number(n), taken };
    }
    shift += 7n;
  }
  throw new FrameError('the table length at the end of the frame is not a varint');
}

/** Reads a frame: its body, and its string table read as far as it is needed. */
export class Unpacker implements Position {
  pos = 0;
  private readonly body: Uint8Array;
  private readonly table: Uint8Array;
  private readonly tablePos: Position = { pos: 0 };
  private readonly strings: string[] = [];
  private depth = 0;

  /** @throws FrameError when `bytes` has no body and table length. */
  constructor(bytes: Uint8Array) {
    const { value: tableLength, taken } = readVarintBack(bytes);
    const bodyEnd = bytes.length - taken - tableLength;
    if (bodyEnd < 0) {
      throw new FrameError('the table is longer than the frame');
    }
    this.body = bytes.subarray(0, bodyEnd);
    this.table = bytes.subarray(bodyEnd, bytes.length - taken);
  }

  get atEnd(): boolean {
    return this.pos === this.body.length;
  }

  private take(len: number): Uint8Array {
    if (len > this.body.length - this.pos) {
      throw new FrameError('the frame ends inside a value');
    }
    const out = this.body.subarray(this.pos, this.pos + len);
    this.pos += len;
    return out;
  }

  private view(len: number): DataView {
    const raw = this.take(len);
    return new DataView(raw.buffer, raw.byteOffset, raw.length);
  }

  uint(): bigint {
    const n = readVarint(this.body, this);
    if (n === undefined) {
      throw new FrameError('a varint cut short or longer than 64 bits');
    }
    return n;
  }

  int(): bigint {
    return unzigzag(this.uint());
  }

  bool(): boolean {
    const [byte] = this.take(1);
    if (byte !== 0 && byte !== 1) {
      throw new FrameError('a bool is a byte, 0 or 1');
    }
    return byte === 1;
  }

  /** The bits of the next `f64`. */
  f64Bits(): bigint {
    return this.view(F64_BYTES).getBigUint64(0, true);
  }

  /** The bits of the next `f32`, widened to an `f64`'s. */
  f32Bits(): bigint {
    return widenBits(this.view(F32_BYTES).getUint32(0, true));
  }

  /** A count of things that each take at least `min` bytes: no more than the body has left. */
  count(min = 1): number {
    const n = this.uint();
    if (n > BigInt(Math.floor((this.body.length - this.pos) / Math.max(min, 1)))) {
      throw new FrameError(`a count of ${n} is more than the frame has bytes for`);
    }
    return Number(n);
  }

  /** The next string, by its index in the table (the table is read as far as the index). */
  str(): string {
    const index = this.uint();
    if (index > BigInt(Number.MAX_SAFE_INTEGER)) {
      throw new FrameError('a string index past the table');
    }
    while (this.strings.length <= Number(index)) {
      this.readTableString();
    }
    return this.strings[Number(index)] as string;
  }

  private readTableString(): void {
    const len = readVarint(this.table, this.tablePos);
    const end = len === undefined ? undefined : BigInt(this.tablePos.pos) + len;
    if (end === undefined || end > BigInt(this.table.length)) {
      throw new FrameError('a string index past the table');
    }
    const bytes = this.table.subarray(this.tablePos.pos, Number(end));
    this.tablePos.pos = Number(end);
    try {
      this.strings.push(utf8.decode(bytes));
    } catch {
      throw new FrameError('a table string is not UTF-8');
    }
  }

  /** Bytes: a count and that many. */
  bytes(): Uint8Array {
    return this.take(this.count());
  }

  /** A packed list of ints, unread: how many, bits each, their bytes. */
  packedInts(): { n: number; width: number; raw: Uint8Array } {
    const count = this.uint();
    const [width] = this.take(1);
    if (width === undefined || width < 1 || width > U64_BITS) {
      throw new FrameError('a list of ints with a bit width outside 1..=64');
    }
    if (count > BigInt(Math.floor(((this.body.length - this.pos) * 8) / width))) {
      throw new FrameError('a list of ints longer than the frame');
    }
    const n = Number(count);
    return { n, width, raw: this.take(packedBytes(n, width)) };
  }

  /** A packed list of floats: the bits of each, as `f64`s. */
  packedFloats(wide: boolean): bigint[] {
    const n = this.count();
    const narrow = wide ? this.bool() : true;
    const width = narrow ? F32_BYTES : F64_BYTES;
    const view = this.view(n * width);
    const out: bigint[] = [];
    for (let i = 0; i < n; i += 1) {
      out.push(narrow ? widenBits(view.getUint32(i * width, true)) : view.getBigUint64(i * width, true));
    }
    return out;
  }

  /** A record starts: where it ends. */
  begin(): number {
    const len = this.count();
    this.enter();
    return this.pos + len;
  }

  /** A list starts: how many items, and where it ends. */
  list(): { n: number; end: number } {
    const n = this.count();
    const len = this.count();
    return { n, end: this.pos + len };
  }

  enter(): void {
    this.depth += 1;
    if (this.depth > MAX_DEPTH) {
      throw new FrameError(`nested deeper than ${MAX_DEPTH} levels`);
    }
  }

  leaveLevel(): void {
    this.depth -= 1;
  }

  /** Steps to `end`, past what is left of a list. */
  skipTo(end: number): void {
    if (this.pos > end || end > this.body.length) {
      throw new FrameError('a list or record whose length is not its content');
    }
    this.pos = end;
  }

  more(end: number): boolean {
    return this.pos < end;
  }
}
