/**
 * Strict roundtrip test: Zod schema → JSON Schema must exactly match the
 * originals in objectiveai-json-schema/.
 *
 * RULES FOR MODIFYING THIS FILE
 * =============================
 *
 * 1. This test MUST NEVER read, load, or deserialize the original JSON schema
 *    files. The harness handles that. Any attempt to read the originals from
 *    this file would defeat the purpose of the test — it would be cheating.
 *    The JSON schemas must be fully reconstructible from the Zod schemas alone.
 *
 * 2. This test MUST be entirely generic. It must work unchanged even if every
 *    existing JSON schema is deleted and replaced with completely different
 *    schemas. No schema-specific special cases, no hardcoded titles, no
 *    xfail lists. If a schema fails, the fix belongs in install-zod.cjs or
 *    in the normalization logic below — never in a per-schema workaround.
 *
 * 3. To make tests pass, you may modify:
 *    - This file (test-zod-roundtrip.test.ts) — conversion logic
 *    - scripts/install-zod.cjs — the Zod code generator
 *
 * 4. You MUST NEVER modify:
 *    - test-zod-roundtrip-harness.ts — the comparison harness
 *    - Any file in objectiveai-json-schema/ — the canonical source of truth
 *
 * DESIGN CHOICE
 * =============
 *
 * This test uses a custom Zod → JSON Schema converter that walks the Zod
 * internal schema tree (_zod.def) directly, rather than using Zod's built-in
 * toJSONSchema(). This produces the correct output format from the start with
 * no cleanup passes needed. It is simpler, smaller, and does not depend on
 * Zod's toJSONSchema output format, which would require extensive
 * post-processing to strip $schema, $defs, additionalProperties:false,
 * propertyNames, safe integer bounds, and other artifacts.
 *
 * This is an information-loss and reconstructibility test. It proves that the
 * Zod schemas contain enough information to perfectly reconstruct the original
 * JSON schemas, with no data lost during the JSON Schema → Zod → JSON Schema
 * round trip.
 */

import { describe, it, expect } from "vitest";
import * as SDK from "../index";
import { ALL_TITLES, assertSchemaMatches } from "./test-zod-roundtrip-harness";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type ZodSchema = any;

/**
 * Detect if a Zod schema is JsonValueSchema (the "any JSON value" type).
 * It's a union of [string, number, boolean, null, array(lazy→self), record(string, lazy→self)]
 * where the lazy getters' results share the same options array identity.
 */
function isJsonValueSchema(schema: ZodSchema): boolean {
  if (schema._zod.def.type !== "union") return false;
  const opts = schema._zod.def.options;
  if (!opts || opts.length !== 6) return false;
  const types = opts.map((o: ZodSchema) => o._zod.def.type);
  if (!["string", "number", "boolean", "null", "array", "record"].every((t) => types.includes(t))) {
    return false;
  }
  // Verify the array element and record value are lazy self-references
  const arr = opts.find((o: ZodSchema) => o._zod.def.type === "array");
  const rec = opts.find((o: ZodSchema) => o._zod.def.type === "record");
  const arrEl = arr?._zod.def.element;
  const recVal = rec?._zod.def.valueType;
  if (arrEl?._zod.def.type !== "lazy" || recVal?._zod.def.type !== "lazy") return false;
  // The lazy getter's result should share the same options (identity check)
  const lazyResult = arrEl._zod.def.getter();
  return lazyResult._zod.def.options === opts;
}

/** "agent.Agent" → "AgentAgentSchema" */
function titleToSchemaName(title: string): string {
  return title
    .split(/[._]/)
    .filter(Boolean)
    .map((s) => s[0].toUpperCase() + s.slice(1))
    .join("") + "Schema";
}

// ---------------------------------------------------------------------------
// Custom Zod → JSON Schema converter
//
// Walks the Zod v4 internal tree (_zod.def) and emits JSON Schema objects
// matching the conventions of the objectiveai-json-schema builder.
// ---------------------------------------------------------------------------

/**
 * Convert a Zod schema to a JSON Schema object.
 */
function convert(
  schema: ZodSchema,
  allTitles: Set<string>,
  rootTitle: string | undefined,
  seen: Set<ZodSchema>,
): Record<string, unknown> {
  if (!seen) seen = new Set();

  // Detect "any JSON value" schemas (JsonValueSchema and its lazy wrappers).
  if (isJsonValueSchema(schema)) {
    const desc = schema.description;
    return desc ? { description: desc } : {};
  }

  // Check for title via .meta() — if it matches a known title, emit $ref
  const meta = typeof schema.meta === "function" ? schema.meta() : undefined;
  const title = meta?.title;
  if (title && allTitles.has(title)) {
    // Self-reference: if we've already seen this schema (entered it), emit $ref
    if (seen.has(schema)) {
      const result: Record<string, unknown> = { $ref: title };
      const desc = schema.description;
      const targetSchema = (SDK as Record<string, ZodSchema>)[titleToSchemaName(title)];
      const targetDesc = targetSchema?.description;
      if (desc && desc !== targetDesc) {
        result.description = desc;
      }
      return result;
    }
    // Non-root ref: emit $ref immediately
    if (title !== rootTitle) {
      const result: Record<string, unknown> = { $ref: title };
      const desc = schema.description;
      const targetSchema = (SDK as Record<string, ZodSchema>)[titleToSchemaName(title)];
      const targetDesc = targetSchema?.description;
      if (desc && desc !== targetDesc) {
        result.description = desc;
      }
      return result;
    }
    // Root schema first encounter: mark as seen and convert inline
    seen = new Set(seen).add(schema);
  }

  return convertInner(schema, allTitles, rootTitle, seen);
}

/** Convert the core of a Zod schema (after $ref check). */
function convertInner(
  schema: ZodSchema,
  allTitles: Set<string>,
  rootTitle: string | undefined,
  seen: Set<ZodSchema>,
): Record<string, unknown> {
  const def = schema._zod.def;
  const type = def.type;
  let result: Record<string, unknown>;

  switch (type) {
    case "string":
      result = convertString(schema);
      break;
    case "number":
      result = convertNumber(schema);
      break;
    case "boolean":
      result = { type: "boolean" };
      break;
    case "null":
      result = { type: "null" };
      break;
    case "literal":
      result = convertLiteral(schema);
      break;
    case "enum":
      result = { type: "string", enum: Object.values(def.entries) };
      break;
    case "array":
      result = convertArray(schema, allTitles, rootTitle, seen);
      break;
    case "object":
      result = convertObject(schema, allTitles, rootTitle, seen);
      break;
    case "record":
      result = convertRecord(schema, allTitles, rootTitle, seen);
      break;
    case "union":
      result = convertUnion(schema, allTitles, rootTitle, seen);
      break;
    case "intersection":
      result = convertIntersection(schema, allTitles, rootTitle, seen);
      break;
    case "nullable":
      result = convertNullable(schema, allTitles, rootTitle, seen);
      break;
    case "optional":
      // optional is handled by the parent object; convert the inner type
      result = convert(def.innerType, allTitles, rootTitle, seen);
      break;
    case "default":
      result = convert(def.innerType, allTitles, rootTitle, seen);
      result = { ...result, default: def.defaultValue };
      break;
    case "lazy":
      result = convert(def.getter(), allTitles, rootTitle, seen);
      break;
    case "never":
      // Used for .strict() catchall — not directly emitted
      result = {};
      break;
    default:
      // Unknown type → any JSON value
      result = {};
      break;
  }

  // Apply description from this level (if not already set by a deeper level)
  const desc = schema.description;
  if (desc && !result.description) {
    result = { ...result, description: desc };
  }

  return result;
}

function convertLiteral(schema: ZodSchema): Record<string, unknown> {
  const values = [...schema._zod.def.values];
  const result = { enum: values };
  // Add type annotation matching the value's JS type
  const litValue = values[0];
  if (typeof litValue === "string") return { type: "string", ...result };
  if (typeof litValue === "number") return { type: "number", ...result };
  if (typeof litValue === "boolean") return { type: "boolean", ...result };
  return result;
}

function convertString(schema: ZodSchema): Record<string, unknown> {
  const result: Record<string, unknown> = { type: "string" };
  // Check for regex pattern and format validators
  if (schema._zod.def.checks) {
    for (const check of schema._zod.def.checks) {
      if (check._zod?.def?.check === "string_format" && check._zod.def.format === "uuid") {
        result.format = "uuid";
      } else if (check._zod?.def?.check === "string_format" && check._zod.def.format === "datetime") {
        result.format = "date-time";
      } else if (check._zod?.def?.check === "string_format" && check._zod.def.pattern) {
        result.pattern = check._zod.def.pattern.source;
      }
    }
  }
  return result;
}

function convertNumber(schema: ZodSchema): Record<string, unknown> {
  const isInt = schema.isInt === true;
  const result: Record<string, unknown> = { type: isInt ? "integer" : "number" };

  // Read min/max directly from checks (accessors clamp to safe integer range)
  if (schema._zod.def.checks) {
    for (const check of schema._zod.def.checks) {
      const cdef = check._zod?.def;
      if (!cdef) continue;
      if (cdef.check === "greater_than" && cdef.inclusive) result.minimum = cdef.value;
      if (cdef.check === "less_than" && cdef.inclusive) result.maximum = cdef.value;
    }
  }

  return result;
}

function convertArray(
  schema: ZodSchema,
  allTitles: Set<string>,
  rootTitle: string | undefined,
  seen: Set<ZodSchema>,
): Record<string, unknown> {
  const result: Record<string, unknown> = { type: "array" };
  if (schema._zod.def.element) {
    result.items = convert(schema._zod.def.element, allTitles, rootTitle, seen);
  }
  // Check for minItems/maxItems in checks
  if (schema._zod.def.checks) {
    for (const check of schema._zod.def.checks) {
      const cdef = check._zod?.def;
      if (cdef?.check === "min_size") result.minItems = cdef.value;
      if (cdef?.check === "max_size") result.maxItems = cdef.value;
    }
  }
  return result;
}

function convertObject(
  schema: ZodSchema,
  allTitles: Set<string>,
  rootTitle: string | undefined,
  seen: Set<ZodSchema>,
): Record<string, unknown> {
  const result: Record<string, unknown> = { type: "object" };
  const shape = schema._zod.def.shape;

  if (shape && Object.keys(shape).length > 0) {
    const properties: Record<string, unknown> = {};
    for (const [key, propSchema] of Object.entries(shape) as [string, ZodSchema][]) {
      // Unwrap optional — the optional wrapper itself just signals optionality
      let inner = propSchema;
      if (inner._zod.def.type === "optional") {
        inner = inner._zod.def.innerType;
      }
      // Check for omitempty in meta before converting (meta is on the
      // outermost wrapper, which may be the optional or the inner schema)
      const propMeta = typeof propSchema.meta === "function" ? propSchema.meta() : undefined;
      const innerMeta = typeof inner.meta === "function" ? inner.meta() : undefined;
      const omitempty = propMeta?.omitempty || innerMeta?.omitempty;
      const prop: Record<string, unknown> = convert(inner, allTitles, rootTitle, seen);
      if (omitempty) {
        prop.omitempty = true;
      }
      properties[key] = prop;
    }
    result.properties = properties;
  }

  // Handle additionalProperties
  const catchall = schema._zod.def.catchall;
  if (catchall) {
    if (catchall._zod.def.type === "never") {
      result.additionalProperties = false;
    } else if (catchall._zod.def.type === "unknown") {
      // `.loose()` sets catchall to z.unknown(), which represents the
      // "any value" position; emit it as `additionalProperties: true`
      // so the round-trip matches the source JSON Schema.
      result.additionalProperties = true;
    } else {
      result.additionalProperties = convert(catchall, allTitles, rootTitle, seen);
    }
  }

  return result;
}

function convertRecord(
  schema: ZodSchema,
  allTitles: Set<string>,
  rootTitle: string | undefined,
  seen: Set<ZodSchema>,
): Record<string, unknown> {
  const result: Record<string, unknown> = { type: "object" };
  if (schema._zod.def.valueType) {
    const valSchema = convert(schema._zod.def.valueType, allTitles, rootTitle, seen);
    // {} (any JSON value) → additionalProperties: true
    result.additionalProperties = Object.keys(valSchema).length === 0 ? true : valSchema;
  }
  return result;
}

function convertUnion(
  schema: ZodSchema,
  allTitles: Set<string>,
  rootTitle: string | undefined,
  seen: Set<ZodSchema>,
): Record<string, unknown> {
  const options = schema._zod.def.options;

  // Check if all options are literals with no individual descriptions or titles → emit as flat enum
  const allLiterals = options.every((o: ZodSchema) => o._zod.def.type === "literal");
  if (allLiterals) {
    const anyHasDesc = options.some((o: ZodSchema) => o.description);
    const anyHasTitle = options.some((o: ZodSchema) => {
      const m = typeof o.meta === "function" ? o.meta() : undefined;
      return m?.variantTitle;
    });
    if (!anyHasDesc && !anyHasTitle) {
      const values = options.flatMap((o: ZodSchema) => [...o._zod.def.values]);
      return { enum: values };
    }
    // Literals with descriptions or titles → anyOf of typed enums
  }

  // Check for nullable pattern: [...variants, null-typed option]
  const nullIdx = options.findIndex((o: ZodSchema) => o._zod.def.type === "null");
  if (nullIdx !== -1) {
    const nonNull = options.filter((_: ZodSchema, i: number) => i !== nullIdx);
    const inner = nonNull.length === 1
      ? convert(nonNull[0], allTitles, rootTitle, seen)
      : { anyOf: nonNull.map((o: ZodSchema) => convert(o, allTitles, rootTitle, seen)) };
    return { anyOf: [inner, { type: "null" }] };
  }

  return { anyOf: options.map((o: ZodSchema) => {
    const converted = convert(o, allTitles, rootTitle, seen);
    const variantMeta = typeof o.meta === "function" ? o.meta() : undefined;
    if (variantMeta?.variantTitle) {
      return { title: variantMeta.variantTitle, ...converted };
    }
    return converted;
  }) };
}

function convertNullable(
  schema: ZodSchema,
  allTitles: Set<string>,
  rootTitle: string | undefined,
  seen: Set<ZodSchema>,
): Record<string, unknown> {
  const inner = convert(schema._zod.def.innerType, allTitles, rootTitle, seen);
  return { anyOf: [inner, { type: "null" }] };
}

function convertIntersection(
  schema: ZodSchema,
  allTitles: Set<string>,
  rootTitle: string | undefined,
  seen: Set<ZodSchema>,
): Record<string, unknown> {
  // Intersection (.and()) is used for $ref + properties (adjacently-tagged enums)
  // and for anyOf + properties (serde flatten).
  // Left side is typically the union/ref, right side is the object with extra properties.
  const left = convert(schema._zod.def.left, allTitles, rootTitle, seen);
  const right = convert(schema._zod.def.right, allTitles, rootTitle, seen);

  // If left is a $ref node, merge right's properties as siblings
  if (left.$ref) {
    const result: Record<string, unknown> = {};
    if (left.description) result.description = left.description;
    if (right.type) result.type = right.type;
    result.$ref = left.$ref;
    if (right.properties) result.properties = right.properties;
    if (right.additionalProperties !== undefined) {
      result.additionalProperties = right.additionalProperties;
    }
    return result;
  }

  // If left has anyOf (serde flatten), combine with right's object part
  if (left.anyOf) {
    const result: Record<string, unknown> = {};
    if (left.description) result.description = left.description;
    result.anyOf = left.anyOf;
    if (right.properties) result.properties = right.properties;
    if (right.type) result.type = right.type;
    return result;
  }

  // Fallback: merge both sides
  return { ...left, ...right };
}

// ---------------------------------------------------------------------------
// Top-level schema conversion
// ---------------------------------------------------------------------------

/**
 * Convert a top-level Zod schema to JSON Schema, including the title and
 * root-level description.
 */
function convertTopLevel(schema: ZodSchema, allTitles: Set<string>): Record<string, unknown> {
  const meta = typeof schema.meta === "function" ? schema.meta() : {};
  const title = meta?.title;
  const desc = schema.description;

  // Convert without $ref-ifying the root itself (but add it to seen for self-ref detection)
  const seen = new Set<ZodSchema>();
  seen.add(schema);
  const result = convertInner(schema, allTitles, title, seen);

  // Add title and description at the top
  const output: Record<string, unknown> = {};
  if (title) output.title = title;
  if (desc) output.description = desc;
  Object.assign(output, result);
  // If description was already in result, the top-level one wins
  if (desc) output.description = desc;

  return output;
}

// ---------------------------------------------------------------------------
// Test runner
// ---------------------------------------------------------------------------

describe("Zod → JSON Schema roundtrip", () => {
  for (const title of ALL_TITLES) {
    const schemaName = titleToSchemaName(title);
    const zodSchema = (SDK as Record<string, ZodSchema>)[schemaName];

    if (!zodSchema) {
      it(`${title} — has matching export: ${schemaName}`, () => {
        expect.fail(`Missing Zod schema export: ${schemaName}`);
      });
      continue;
    }

    it(title, () => {
      const converted = convertTopLevel(zodSchema, ALL_TITLES);
      assertSchemaMatches(title, converted);
    });
  }
});
