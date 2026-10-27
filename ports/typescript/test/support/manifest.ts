// The shape of conformance/cases.json, and the files its cases point to.

import { readFileSync } from 'node:fs';
import { fromHex } from './hex.js';

export const CONFORMANCE = new URL('../../../../../conformance/', import.meta.url);

export type JsonValue =
  | null
  | boolean
  | number
  | string
  | JsonValue[]
  | { [key: string]: JsonValue };

/** A model value in the manifest's tagged JSON. */
export interface ModelJson {
  t: string;
  v?: JsonValue;
  bits?: string;
  hex?: string;
  kind?: string;
  id?: string;
}

export interface FrameJson {
  schema: string;
  shape: string;
  value: ModelJson;
  bytes?: string;
  bytes_file?: string;
  also_read_by?: { schema: string; shape: string; value: ModelJson }[];
}

export interface IoJson {
  bytes?: string;
  bytes_file?: string;
  messages: string[];
  end: string;
}

export interface DiagnosticJson {
  code: string;
  line: number;
  col: number;
  path: string;
}

export interface ErrorJson {
  form: 'text' | 'text_schema' | 'binary' | 'frame' | 'io';
  input: string;
  line?: number;
  col?: number;
  kind?: string;
  frames_before?: number;
  schema?: string;
  shape?: string;
  diagnostics?: DiagnosticJson[];
}

export interface Case {
  name: string;
  text?: string;
  text_is_written?: boolean;
  value?: ModelJson;
  binary?: string;
  binary_file?: string;
  binary_is_encoded?: boolean;
  frame?: FrameJson;
  io?: IoJson;
  error?: ErrorJson;
}

export function loadCases(): Case[] {
  const text = readFileSync(new URL('cases.json', CONFORMANCE), 'utf8');
  return JSON.parse(text) as Case[];
}

/** A case's raw bytes: its hex in the manifest, or its file when it is long. */
export function bytesOf(hex: string | undefined, file: string | undefined): Uint8Array {
  if (hex !== undefined) {
    return fromHex(hex);
  }
  if (file === undefined) {
    throw new Error('a form with neither hex nor a file');
  }
  return new Uint8Array(readFileSync(new URL(file, CONFORMANCE)));
}
