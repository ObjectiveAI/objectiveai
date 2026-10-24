// What a `list tools` asks.

import type { z } from "zod";
import { PaginatedRequestParamsSchema } from "@modelcontextprotocol/core";
import { json } from "../../../../wire/encode.js";

/**
 * `tools/list`: the params, MCP's own, or `null` for none — a
 * request with no params and one with no cursor differ to a server,
 * so the absence is kept.
 */
export type Request = z.infer<typeof PaginatedRequestParamsSchema> | null;

/** Its JSON, and nothing in front of it: the tag that says which exchange belongs to whichever frame carries it. */
export function encode(request: Request): Uint8Array {
  return json(request);
}
