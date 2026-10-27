/** A value that breaks the model's rules (a duplicate key, a bad reference). */
export class InvalidValueError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'InvalidValueError';
  }
}
