// What a `call tool` asks.

import type { z } from "zod";
import { CallToolRequestParamsSchema } from "@modelcontextprotocol/core";
import { json } from "../../../../wire/encode.js";

/** `tools/call`: the params, MCP's own — the tool's name and its arguments. */
export type Request = z.infer<typeof CallToolRequestParamsSchema>;

/** Its JSON, and nothing in front of it: the tag that says which exchange belongs to whichever frame carries it. */
export function encode(request: Request): Uint8Array {
  return json(request);
}
