import cn from "classnames";
import { memo, useMemo } from "react";

/**
 * Pretty JSON with WRAP-SAFE indentation — the star is wrapping, not
 * newlines: each pretty-printed line renders as its own block whose
 * wrapped continuations hang at that line's indentation + 2ch instead
 * of falling back to column zero (a plain `pre-wrap` blob restarts
 * wrapped lines at the container edge, which is what made long
 * strings ugly).
 *
 * Per line: `padding-left: (indent + 2)ch` positions the whole block,
 * `text-indent: -2ch` pulls only the FIRST visual line back to the
 * true indentation — so continuations land 2ch deeper, visibly a
 * wrap, at any nesting depth (the hang is measured from each line's
 * OWN leading whitespace, so recursion depth costs nothing).
 * `ch` is exact because the container is monospace. When Chromium
 * ships `text-indent: hanging each-line`, one declaration on a plain
 * `pre` replaces all of this.
 *
 * Values are pretty-printed with string-embedded JSON EXPANDED
 * recursively (depth-capped): a string value that parses to an
 * object/array — tool-call arguments, inline agents — renders as the
 * parsed structure, not an escaped one-liner. That is a DISPLAY
 * transform: the shown text is no longer the literal outer document,
 * so copy affordances should copy the caller's original text.
 *
 * ESCAPED NEWLINES inside string tokens render as REAL line breaks
 * ([`unescapeNewlines`]): the continuation sits at the line's own
 * (key) indentation — 2ch SHALLOWER than a wrap's hang, which is
 * what tells "the string contains a newline" apart from "this line
 * wrapped". The quotes stay, and NO other escape is touched (`\t`,
 * `\"`, and a literal `\\n` all render escaped).
 */

/** How deep string-embedded JSON expansion recurses before giving up
 * (a runaway guard, not a tuning knob). */
const MAX_EXPAND_DEPTH = 4;

/** Parse a string that LOOKS like embedded JSON (`{`/`[` after trim)
 * into its object/array; `undefined` for everything else — scalars
 * like `"123"`/`"true"` stay strings. */
function parseEmbedded(value: string): unknown {
  const trimmed = value.trimStart();
  if (!trimmed.startsWith("{") && !trimmed.startsWith("[")) {
    return undefined;
  }
  try {
    const parsed: unknown = JSON.parse(value);
    return typeof parsed === "object" && parsed !== null
      ? parsed
      : undefined;
  } catch {
    return undefined;
  }
}

/** Recursively replace string-embedded JSON values with their parsed
 * structures, so one final stringify pretty-prints every level. */
function expandEmbedded(value: unknown, depth: number): unknown {
  if (typeof value === "string") {
    if (depth >= MAX_EXPAND_DEPTH) return value;
    const embedded = parseEmbedded(value);
    return embedded === undefined
      ? value
      : expandEmbedded(embedded, depth + 1);
  }
  if (Array.isArray(value)) {
    return value.map((item) => expandEmbedded(item, depth));
  }
  if (typeof value === "object" && value !== null) {
    return Object.fromEntries(
      Object.entries(value).map(([key, item]) => [
        key,
        expandEmbedded(item, depth),
      ]),
    );
  }
  return value;
}

/** The display text: a string value is itself sniffed for embedded
 * JSON (raw text passes through verbatim); everything else
 * pretty-prints with embedded strings expanded. */
function displayText(value: unknown): string {
  if (typeof value === "string") {
    const embedded = parseEmbedded(value);
    if (embedded === undefined) return value;
    return JSON.stringify(expandEmbedded(embedded, 1), null, 2);
  }
  return (
    JSON.stringify(expandEmbedded(value, 0), null, 2) ?? String(value)
  );
}

/** Replace the two-character `\n` escape inside STRING tokens with a
 * real line break, the continuation indented at the line's own (key)
 * indentation. A character scanner, not a regex, because escape
 * sequences must be consumed as units: `\\n` is a literal backslash
 * followed by `n` — NOT a newline — and every non-`\n` escape passes
 * through untouched. Lines of pretty-printed JSON are self-contained
 * (stringify never emits a raw newline inside a token), so scanning
 * per line is sound. */
function unescapeNewlines(text: string): string {
  return text
    .split("\n")
    .map((line) => {
      const indent = line.length - line.trimStart().length;
      const brk = "\n" + " ".repeat(indent);
      let out = "";
      let inString = false;
      for (let i = 0; i < line.length; i++) {
        const c = line[i];
        if (inString && c === "\\") {
          const next = line[i + 1] ?? "";
          out += next === "n" ? brk : c + next;
          i++;
          continue;
        }
        if (c === '"') inString = !inString;
        out += c;
      }
      return out;
    })
    .join("\n");
}

export const JsonBlock = memo(function JsonBlock({
  value,
  className,
}: {
  /** The value to display. A string is sniffed for embedded JSON
   * (non-JSON strings render verbatim, still wrap-safe); everything
   * else pretty-prints. */
  value: unknown;
  /** Container styling (font size, color, scroll bounds). Monospace
   * is baked in — the hanging indent's `ch` math needs it. */
  className?: string;
}) {
  const lines = useMemo(
    () => unescapeNewlines(displayText(value)).split("\n"),
    [value],
  );
  return (
    <div className={cn("font-mono", className)}>
      {lines.map((line, i) => {
        const trimmed = line.trimStart();
        const indent = line.length - trimmed.length;
        return (
          <div
            key={i}
            className={cn("whitespace-pre-wrap", "break-words")}
            style={{
              paddingLeft: `${indent + 2}ch`,
              textIndent: "-2ch",
            }}
          >
            {trimmed === "" ? " " : trimmed}
          </div>
        );
      })}
    </div>
  );
});
