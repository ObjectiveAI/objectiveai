// What a `read resource` asks.

import type { z } from "zod";
import { ReadResourceRequestParamsSchema } from "@modelcontextprotocol/core";
import { json } from "../../../../wire/encode.js";

/** `resources/read`: the params, MCP's own — the resource's URI. */
export type Request = z.infer<typeof ReadResourceRequestParamsSchema>;

/** Its JSON, and nothing in front of it: the tag that says which exchange belongs to whichever frame carries it. */
export function encode(request: Request): Uint8Array {
  return json(request);
}
