/** A frame that is not what its shape says, or a shape this port cannot read by. */
export class FrameError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'FrameError';
  }
}
