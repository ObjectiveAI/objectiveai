// A JSON value, shown so it reads: pretty, each line hanging under its
// own indent, strings that hold JSON opened up, and escaped newlines
// broken. The idea is the old viewer's `JsonBlock`.

/** How deep strings holding JSON are opened. */
const DEPTH = 4;

/** `value` with every string that holds a JSON object or array parsed, `depth` levels down. */
function opened(value: unknown, depth: number): unknown {
  if (depth <= 0) return value;
  if (typeof value === "string") {
    const t = value.trim();
    if ((t.startsWith("{") && t.endsWith("}")) || (t.startsWith("[") && t.endsWith("]"))) {
      try {
        return opened(JSON.parse(t), depth - 1);
      } catch {
        return value;
      }
    }
    return value;
  }
  if (Array.isArray(value)) return value.map((v) => opened(v, depth));
  if (value !== null && typeof value === "object") {
    return Object.fromEntries(Object.entries(value).map(([k, v]) => [k, opened(v, depth)]));
  }
  return value;
}

/**
 * A block showing `value`.
 *
 * The rules:
 *
 * 1. Strings holding a JSON object or array are parsed and shown as
 *    JSON, `DEPTH` levels down.
 * 2. The value is pretty-printed, two spaces an indent, one element a
 *    line.
 * 3. A line that wraps hangs two characters under its own indent.
 * 4. An escaped newline or tab inside a string is shown as one: the
 *    line breaks there, and the rest keeps the line's indent.
 */
export function jsonBlock(value: unknown): HTMLElement {
  const block = document.createElement("div");
  block.className = "json";
  // Rules 1 and 2.
  const text = JSON.stringify(opened(value, DEPTH), null, 2) ?? String(value);
  for (const line of text.split("\n")) {
    const indent = line.length - line.trimStart().length;
    const row = document.createElement("div");
    // Rule 3.
    row.style.paddingLeft = `${indent + 2}ch`;
    row.style.textIndent = "-2ch";
    // Rule 4.
    row.textContent = line.trimStart().replaceAll("\\n", "\n").replaceAll("\\t", "    ");
    block.append(row);
  }
  return block;
}

/** `text` as JSON when it is a JSON object or array, else `undefined`. */
export function asJson(text: string): unknown {
  const t = text.trim();
  if (!(t.startsWith("{") || t.startsWith("["))) return undefined;
  try {
    return JSON.parse(t);
  } catch {
    return undefined;
  }
}
