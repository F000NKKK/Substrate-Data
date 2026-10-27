/** Somewhere bytes are appended. */
export interface ByteSink {
  push(byte: number): void;
}

/** A growable byte buffer. */
export class ByteBuffer implements ByteSink {
  private bytes = new Uint8Array(256);
  private length = 0;

  push(byte: number): void {
    this.reserve(1);
    this.bytes[this.length] = byte;
    this.length += 1;
  }

  pushAll(more: Uint8Array): void {
    this.reserve(more.length);
    this.bytes.set(more, this.length);
    this.length += more.length;
  }

  finish(): Uint8Array {
    return this.bytes.slice(0, this.length);
  }

  private reserve(extra: number): void {
    if (this.length + extra <= this.bytes.length) {
      return;
    }
    const grown = new Uint8Array(Math.max(this.bytes.length * 2, this.length + extra));
    grown.set(this.bytes.subarray(0, this.length));
    this.bytes = grown;
  }
}
