// The one text schema the vectors use (`limits`), read the way a schema layer
// would, to check the diagnostics: code, position, path. This is test support,
// not part of the library: the port has no schema system.
//
//   { copies: u32 in 1..=99, owner: "@user:<u64>", file: "@file:<text>" }

import { lineColOf, parseDocument, type Span, type Value } from '../../src/index.js';
import type { DiagnosticJson } from './manifest.js';

const MAX_COPIES = 99;
const MIN_COPIES = 1;
const FIELDS = ['copies', 'owner', 'file'] as const;
const KIND_NAME = /^[a-z][a-z0-9_]*$/;
const U64 = /^(0|[1-9][0-9]*)$/;
const U64_MAX = (1n << 64n) - 1n;

interface Problem {
  readonly code: string;
  readonly at: number;
  readonly path: string;
}

export function diagnoseLimits(input: string): DiagnosticJson[] {
  const { value, spans } = parseDocument(input);
  const problems: Problem[] = [];
  if (value.kind !== 'map') {
    problems.push({ code: 'SD0002', at: spans.start, path: '(root)' });
  } else {
    value.entries.forEach(([key, v], i) => {
      problems.push(...checkEntry(key, v, spans.children[i] as Span));
    });
    for (const name of FIELDS) {
      if (!value.entries.some(([key]) => key === name)) {
        problems.push({ code: 'SD0006', at: spans.start, path: '(root)' });
      }
    }
  }
  return problems.map((p) => ({ code: p.code, ...lineColOf(input, p.at), path: p.path }));
}

function checkEntry(key: string, value: Value, span: Span): Problem[] {
  const problem = (code: string, at = span.start): Problem[] => [{ code, at, path: key }];
  switch (key) {
    case 'copies':
      return checkCopies(value, problem);
    case 'owner':
      return checkReference(value, 'user', true, problem);
    case 'file':
      return checkReference(value, 'file', false, problem);
    default:
      return problem('SD0005', span.keyStart);
  }
}

type Report = (code: string, at?: number) => Problem[];

function checkCopies(value: Value, report: Report): Problem[] {
  if (value.kind !== 'int') {
    return report('SD0002');
  }
  const inRange = value.value >= BigInt(MIN_COPIES) && value.value <= BigInt(MAX_COPIES);
  return inRange ? [] : report('SD0007');
}

function checkReference(value: Value, kind: string, numeric: boolean, report: Report): Problem[] {
  if (value.kind !== 'str') {
    return report('SD0002');
  }
  const text = value.value;
  if (!text.startsWith('@')) {
    return report('SD0003');
  }
  const [refKind = '', ...rest] = text.slice(1).split(':');
  const id = rest.join(':');
  if (rest.length === 0 || !KIND_NAME.test(refKind) || id.length === 0) {
    return report('SD0008');
  }
  if (refKind !== kind) {
    return report('SD0004');
  }
  const idOk = !numeric || (U64.test(id) && BigInt(id) <= U64_MAX);
  return idOk ? [] : report('SD0010');
}
