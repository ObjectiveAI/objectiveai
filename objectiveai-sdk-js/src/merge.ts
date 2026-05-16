export function merge<T extends {}>(
  a: T,
  b: T,
  combine?: (a: T, b: T) => [T, boolean]
): [T, boolean];
export function merge<T extends {}>(
  a: T | null,
  b: T | null,
  combine?: (a: T, b: T) => [T, boolean]
): [T | null, boolean];
export function merge<T extends {}>(
  a: T | undefined,
  b: T | undefined,
  combine?: (a: T, b: T) => [T, boolean]
): [T | undefined, boolean];
export function merge<T extends {}>(
  a: T | null | undefined,
  b: T | null | undefined,
  combine?: (a: T, b: T) => [T, boolean]
): [T | null | undefined, boolean];
export function merge<T extends {}>(
  a: T | null | undefined,
  b: T | null | undefined,
  combine?: (a: T, b: T) => [T, boolean]
): [T | null | undefined, boolean] {
  if (a !== null && a !== undefined && b !== null && b !== undefined) {
    return combine ? combine(a, b) : [a, false];
  } else if (a !== null && a !== undefined) {
    return [a, false];
  } else if (b !== null && b !== undefined) {
    return [b, true];
  } else if (a === null || b === null) {
    return [null, false];
  } else {
    return [undefined, false];
  }
}

export function mergedString(a: string, b: string): [string, boolean] {
  return b === "" ? [a, false] : [a + b, true];
}

export function mergedNumberArray(
  a: number[],
  b: number[],
): [number[], boolean] {
  if (a.length === b.length) {
    for (let i = 0; i < a.length; i++) {
      if (a[i] !== b[i]) return [b, true];
    }
    return [a, false];
  }
  return [b, true];
}
