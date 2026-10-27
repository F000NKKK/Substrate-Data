// A reference's two parts: its kind and its id.

const KIND = /^[a-z][a-z0-9_]*$/;
const ID_FORBIDDEN = /[\p{White_Space}\p{Cc}]/u;

/** Whether `name` is a reference kind: lower case ASCII, `[a-z][a-z0-9_]*`. */
export function isKindName(name: string): boolean {
  return KIND.test(name);
}

/** Whether `id` may be a reference's id: not empty, no whitespace, no control characters. */
export function isRefId(id: string): boolean {
  return id.length > 0 && !ID_FORBIDDEN.test(id);
}
