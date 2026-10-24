// What a `call tool` answers.

import { z } from "zod";
import { CallToolResultSchema } from "@modelcontextprotocol/core";
import { DecodeError, jsonOf, splitTag } from "../../../../wire/decode.js";
import { ErrorData } from "../../error_data.js";

/**
 * What answers a `tools/call`: the result, or the server's error.
 * Exactly one, then the channel's finish. A payload leads with one
 * byte saying which — `0` for `result`, `1` for `error` — and the
 * rest is that variant's JSON. A tool that failed in its own terms
 * is a `result` with `isError`; an `error` is the server's.
 */
export type Frame = { readonly result: z.infer<typeof CallToolResultSchema> } | { readonly error: ErrorData };
// Annotated rather than inferred: MCP's result types name a `JSONValue`
// the package does not export, which a declaration could not refer to.
export const Frame: z.ZodType<Frame> = z.union([
  /** The server's answer. Tag `0`. */
  z.strictObject({ result: CallToolResultSchema }),
  /** The server's error, whole. Tag `1`. */
  z.strictObject({ error: ErrorData }),
]);

const RESULT = 0;
const ERROR = 1;

/**
 * A frame out of its payload: the tag byte, then the variant's own
 * JSON, parsed by MCP's schema. A tag that names no variant is the
 * `unexpected_tag` error.
 */
export function decode(bytes: Uint8Array): Frame {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case RESULT:
      return { result: CallToolResultSchema.parse(jsonOf(rest)) };
    case ERROR:
      return { error: ErrorData.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
