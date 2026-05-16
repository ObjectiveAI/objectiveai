export function rounded<T>(value: T): T {
  if (typeof value === "number") {
    if (value === 0 || !Number.isFinite(value)) return value;
    return parseFloat(parseFloat(value.toPrecision(12)).toPrecision(8)) as T;
  }
  if (Array.isArray(value)) {
    return value.map(rounded) as T;
  }
  if (value !== null && typeof value === "object") {
    const obj = value as Record<string, unknown>;
    const result: Record<string, unknown> = {};
    for (const k in obj) {
      result[k] = rounded(obj[k]);
    }
    return result as T;
  }
  return value;
}
