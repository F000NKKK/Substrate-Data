/**
 * A finite float in the shortest spelling that reads back as exactly it and
 * as a float: `1.0`, `0.1`, `-0.0`, `1e-7`, `1e16` (never `10000000000000000`,
 * which would read back as an integer).
 */
export function floatText(x: number): string {
  if (x === 0) {
    return Object.is(x, -0) ? '-0.0' : '0.0';
  }
  const sign = x < 0 ? '-' : '';
  const magnitude = Math.abs(x);
  const [mantissa = '', exponentText = '0'] = magnitude.toExponential().split('e');
  const exponent = Number(exponentText);
  if (magnitude >= 1e16 || magnitude < 1e-4) {
    return `${sign}${mantissa}e${exponent}`;
  }
  return sign + positional(mantissa.replace('.', ''), exponent);
}

/** `digits` (the significant digits) times 10^(exponent - digits.length + 1), written out. */
function positional(digits: string, exponent: number): string {
  if (exponent < 0) {
    return `0.${'0'.repeat(-exponent - 1)}${digits}`;
  }
  const whole = exponent + 1;
  if (digits.length <= whole) {
    return `${digits}${'0'.repeat(whole - digits.length)}.0`;
  }
  return `${digits.slice(0, whole)}.${digits.slice(whole)}`;
}
