import type { Value } from '../model/value.js';
import { base64Encode } from './base64.js';
import { floatText } from './float-text.js';

const TEXT_ESCAPES: Readonly<Record<string, string>> = {
  '"': '\\"',
  '\\': '\\\\',
  '\n': '\\n',
  '\r': '\\r',
  '\t': '\\t',
};

/** The string a typed value is written as: `@kind:id`, `@bytes:<base64>`. */
export function typedSpelling(value: Value): string | undefined {
  if (value.kind === 'ref') {
    return `@${value.refKind}:${value.id}`;
  }
  if (value.kind === 'bytes') {
    return `@bytes:${base64Encode(value.value)}`;
  }
  return undefined;
}

/** `value` as compact text; a ref or bytes as its typed string. */
export function writeText(value: Value): string {
  switch (value.kind) {
    case 'null':
      return 'null';
    case 'bool':
      return value.value ? 'true' : 'false';
    case 'int':
      return value.value.toString();
    case 'float':
      return Number.isFinite(value.value) ? floatText(value.value) : 'null';
    case 'str':
      return quote(value.value);
    case 'list':
      return `[${value.items.map(writeText).join(',')}]`;
    case 'map':
      return `{${value.entries.map(([k, v]) => `${quote(k)}:${writeText(v)}`).join(',')}}`;
    case 'ref':
    case 'bytes':
      return quote(typedSpelling(value) ?? '');
  }
}

/** `s` as a JSON string literal. */
function quote(s: string): string {
  let out = '"';
  for (const ch of s) {
    out += TEXT_ESCAPES[ch] ?? control(ch);
  }
  return `${out}"`;
}

function control(ch: string): string {
  const code = ch.codePointAt(0) ?? 0;
  return code < 0x20 ? `\\u${code.toString(16).padStart(4, '0')}` : ch;
}
