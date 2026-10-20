// One kind of chunk: `tool_response`.

import { z } from "zod";
import { CallToolResultSchema } from "@modelcontextprotocol/core";

/**
 * The result of one tool call.
 *
 * Arrives whole, unlike the assistant chunks: a tool either returned or
 * it did not, so there is nothing to stream in pieces.
 *
 * The result is MCP's own `CallToolResult`, flattened, so what an MCP
 * server returned passes through verbatim — content blocks, structured
 * content, `isError` and `_meta` included — rather than being
 * re-encoded into a shape of ours that would lose some of it.
 */
export const ToolResponseChunk = z.looseObject({
  /** Which kind this is. */
  type: z.literal("tool_response"),
  /** The tool call this is part of, when the loop ran it inside one; absent at the top. */
  parent_tool_call_id: z.string().optional(),
  /** The call this answers. Ours, not MCP's: a `CallToolResult` carries no id at all, because in MCP it is the payload of a JSON-RPC response and the request id does the correlating from the envelope. A stream has no envelope, and results may arrive in a different order than the calls were made, so the link has to be here. */
  id: z.string(),
  /** MCP's `CallToolResult`, flattened in; loose, so that what a server put beside its members — `resultType` among them — passes through. */
  ...CallToolResultSchema.shape,
});
export type ToolResponseChunk = z.infer<typeof ToolResponseChunk>;
