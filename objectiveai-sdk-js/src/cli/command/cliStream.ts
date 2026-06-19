import { z } from "zod";

/**
 * Async-iterable wrapper over the `cli_command` line stream produced
 * by `invokeCliRequest`. Used by the generated viewer execute
 * functions; plugin authors normally receive one rather than
 * constructing one.
 *
 * Each raw line is zod-parsed with `schema` — typically
 * `z.union([CliOutputErrorSchema, <ResponseSchema>])`, so error
 * envelopes and response values surface as plain union members
 * rather than thrown exceptions.
 *
 * Before parsing, externally-tagged aggregate layers are unwrapped:
 * the cli prints the aggregate `ResponseItem` wire shape (e.g.
 * `{"Config":{"Viewer":{"Get":{}}}}`), so single-key object wrappers
 * are peeled until the schema parses — the JS mirror of
 * `extract_leaf` in `objectiveai-cli/src/executor.rs`.
 *
 * The host's synthetic `{"type":"end"}` terminator line is consumed
 * (it ends iteration) and never yielded.
 */
export class CliStream<T> implements AsyncIterable<T> {
  private readonly source: AsyncIterable<unknown>;
  private readonly schema: z.ZodType<T>;

  constructor(source: AsyncIterable<unknown>, schema: z.ZodType<T>) {
    this.source = source;
    this.schema = schema;
  }

  async *[Symbol.asyncIterator](): AsyncIterator<T> {
    for await (const line of this.source) {
      if (isEndMarker(line)) {
        // The underlying iterable terminates itself after the end
        // marker; skipping here just keeps the marker out of the
        // typed stream.
        continue;
      }
      yield parseUnwrapping(this.schema, line);
    }
  }

  /** Collect every remaining item. */
  async toArray(): Promise<T[]> {
    const items: T[] = [];
    for await (const item of this) {
      items.push(item);
    }
    return items;
  }

  /**
   * Resolve the first item and discard the rest of the stream
   * (releasing the underlying message listener). `undefined` when the
   * stream ends without yielding — a unary command that printed
   * nothing before the end marker.
   */
  async first(): Promise<T | undefined> {
    const iter = this[Symbol.asyncIterator]();
    try {
      const result = await iter.next();
      return result.done ? undefined : result.value;
    } finally {
      await iter.return?.(undefined);
    }
  }
}

/** The host's synthetic `{"type":"end"}` stream terminator. */
function isEndMarker(line: unknown): boolean {
  return (
    typeof line === "object" &&
    line !== null &&
    (line as { type?: unknown }).type === "end"
  );
}

/**
 * Parse `value` with `schema`, unwrapping externally-tagged
 * single-key object layers (`{"Agents":{"Spawn":…}}`) until the
 * schema accepts — the JS mirror of the cli's `extract_leaf`. Throws
 * the ZodError for the ORIGINAL value when no layer parses, so the
 * error always shows the full wire shape.
 */
function parseUnwrapping<T>(schema: z.ZodType<T>, value: unknown): T {
  let current = value;
  for (;;) {
    const result = schema.safeParse(current);
    if (result.success) return result.data;
    if (
      current !== null &&
      typeof current === "object" &&
      !Array.isArray(current)
    ) {
      const keys = Object.keys(current as Record<string, unknown>);
      if (keys.length === 1) {
        current = (current as Record<string, unknown>)[keys[0]];
        continue;
      }
    }
    return schema.parse(value);
  }
}
