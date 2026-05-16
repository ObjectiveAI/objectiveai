/**
 * Strict roundtrip test harness for Zod JSON Schema validation.
 *
 * THIS FILE MUST NEVER BE MODIFIED.
 *
 * This harness is purposefully strict. It loads the original JSON schemas from
 * objectiveai-json-schema/ exactly as they are on disk — no normalization, no
 * massaging, no xfail. The original schema is treated as the canonical source
 * of truth and is never altered.
 *
 * The contract is simple: the caller passes a schema title and an object. This
 * harness loads the original, serializes both sides using the canonical key
 * ordering from the JSON schema builder (objectiveai-json-schema/builder/),
 * and compares the serialized strings for exact equality.
 *
 * Key ordering rules (matching the Rust builder):
 *   - Inside "properties": keys are sorted alphabetically.
 *   - Outside "properties": keys are sorted by KEYWORD_ORDER, with any
 *     unknown keys placed at the end.
 *
 * If a test fails, the fix belongs in the caller's conversion/normalization
 * logic or in the Zod code generator — never in this file.
 */

import * as fs from "fs";
import * as path from "path";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const SCHEMA_DIR = path.resolve(__dirname, "../../../objectiveai-json-schema");

// Canonical key ordering for JSON Schema keywords.
// Matches KEYWORD_ORDER in objectiveai-json-schema/builder/src/main.rs.
export const KEYWORD_ORDER = [
  "title",
  "description",
  "type",
  "enum",
  "anyOf",
  "$ref",
  "properties",
  "additionalProperties",
  "items",
  "minItems",
  "maxItems",
  "minimum",
  "maximum",
  "pattern",
  "format",
  "default",
  "omitempty",
];

const KEYWORD_RANK = new Map(KEYWORD_ORDER.map((kw, i) => [kw, i]));
const UNKNOWN_RANK = KEYWORD_ORDER.length;

/**
 * Recursively reorder keys to match the Rust builder's canonical order.
 *
 * - Inside `properties`: keys (field names) are sorted alphabetically.
 * - Outside `properties`: keys are sorted by KEYWORD_ORDER, with
 *   unknown keys placed at the end (preserving their relative order).
 */
export function orderKeys(value: unknown, insideProperties: boolean): unknown {
  if (value === null || typeof value !== "object") return value;
  if (Array.isArray(value)) {
    return value.map((v) => orderKeys(v, false));
  }

  // Recurse first
  const recursed: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(value)) {
    recursed[k] = orderKeys(v, k === "properties");
  }

  // Sort this level's keys
  const entries = Object.entries(recursed);
  if (insideProperties) {
    entries.sort(([a], [b]) => a.localeCompare(b));
  } else {
    entries.sort(([a], [b]) => {
      const ra = KEYWORD_RANK.has(a) ? KEYWORD_RANK.get(a)! : UNKNOWN_RANK;
      const rb = KEYWORD_RANK.has(b) ? KEYWORD_RANK.get(b)! : UNKNOWN_RANK;
      return ra - rb;
    });
  }

  const result: Record<string, unknown> = {};
  for (const [k, v] of entries) {
    result[k] = v;
  }
  return result;
}

/**
 * Serialize a schema object to a canonical JSON string.
 *
 * Applies the builder's key ordering, then pretty-prints with 2-space
 * indent (matching `serde_json::to_string_pretty`).
 */
export function serialize(schema: unknown): string {
  const ordered = orderKeys(schema, false);
  return JSON.stringify(ordered, null, 2);
}

/**
 * Load all JSON schemas from objectiveai-json-schema/ exactly as-is.
 *
 * Returns a Map mapping each schema's `title` to its raw parsed content.
 * No normalization is applied.
 */
export function loadOriginalJsonSchemas(): Map<string, Record<string, unknown>> {
  const schemas = new Map<string, Record<string, unknown>>();
  const files = fs.readdirSync(SCHEMA_DIR).filter((f) => f.endsWith(".json"));
  files.sort();
  for (const file of files) {
    const content = JSON.parse(
      fs.readFileSync(path.join(SCHEMA_DIR, file), "utf-8")
    );
    if (content.title) {
      schemas.set(content.title, content);
    }
  }
  return schemas;
}

// ---------------------------------------------------------------------------
// Preloaded schemas — available to importers
// ---------------------------------------------------------------------------

export const ORIGINAL_SCHEMAS = loadOriginalJsonSchemas();
export const ALL_TITLES = new Set(ORIGINAL_SCHEMAS.keys());

/**
 * Assert that a converted schema exactly matches the original on disk.
 *
 * Both the original and `converted` are serialized using the canonical
 * key ordering before comparison, so key order differences alone will not
 * cause spurious failures — but every key, value, and nesting level must
 * match exactly.
 *
 * @param title - The schema title. Must exist in ORIGINAL_SCHEMAS.
 * @param converted - The caller's Zod-derived JSON Schema object,
 *   already normalized however the caller sees fit.
 * @throws If the serialized forms differ.
 * @throws If `title` is not found in the original schemas.
 */
export function assertSchemaMatches(title: string, converted: Record<string, unknown>): void {
  const original = ORIGINAL_SCHEMAS.get(title);
  if (!original) {
    throw new Error(`Schema title not found in originals: '${title}'`);
  }
  const expectedStr = serialize(original);
  const actualStr = serialize(converted);
  if (actualStr !== expectedStr) {
    throw new Error(
      `Schema mismatch for '${title}':\n` +
        `\n--- Expected (original from objectiveai-json-schema/) ---\n` +
        `${expectedStr}\n` +
        `\n--- Got (Zod-derived) ---\n` +
        `${actualStr}`
    );
  }
}
