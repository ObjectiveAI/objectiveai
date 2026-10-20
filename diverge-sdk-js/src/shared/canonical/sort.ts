// Every object key sorted, at every depth.

const ENCODER = new TextEncoder();

/**
 * Compare two strings by their UTF-8 bytes, which is how the Rust
 * sorts keys and names: JavaScript's own order is by UTF-16 unit,
 * which differs for characters beyond the basic plane.
 */
export function bytewise(a: string, b: string): number {
  const left = ENCODER.encode(a);
  const right = ENCODER.encode(b);
  const length = Math.min(left.length, right.length);
  for (let i = 0; i < length; i += 1) {
    const difference = (left[i] as number) - (right[i] as number);
    if (difference !== 0) {
      return difference;
    }
  }
  return left.length - right.length;
}

/**
 * `value` with every object key sorted bytewise, at every depth —
 * inside nested objects and inside arrays alike — as a new value;
 * the one given is not touched. Array ELEMENT order is deliberately
 * untouched: an array is ordered data, and reordering it would
 * change what the value says, not just how it was spelled.
 */
export function sortObjectKeys(value: unknown): unknown {
  if (Array.isArray(value)) {
    return value.map(sortObjectKeys);
  }
  if (typeof value === "object" && value !== null) {
    const object = value as Record<string, unknown>;
    const sorted: Record<string, unknown> = {};
    for (const key of Object.keys(object).sort(bytewise)) {
      sorted[key] = sortObjectKeys(object[key]);
    }
    return sorted;
  }
  return value;
}
