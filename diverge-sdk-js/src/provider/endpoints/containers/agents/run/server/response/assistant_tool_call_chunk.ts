// One kind of chunk: `assistant_tool_call`.

import { z } from "zod";
import { RequestMetaSchema } from "@modelcontextprotocol/core";

/**
 * The model calling a tool.
 *
 * A delta, like the text chunks around it. Providers stream tool
 * arguments in fragments, and this chunk carries them as they come:
 * consecutive tool call chunks bearing the same `id` continue one call,
 * their `arguments` concatenating in the order sent; a chunk with a new
 * `id` is a new call. The concatenation is the call's arguments as JSON
 * text, complete only when the last fragment has arrived.
 *
 * The fields are this chunk's own, spelled bare — they no longer ride
 * MCP's `CallToolRequestParams`, which is what made a delta form
 * possible: a fragment of a JSON object is not a JSON object, but a
 * fragment of a string is a string.
 */
export const AssistantToolCallChunk = z.object({
  /** Which kind this is. */
  type: z.literal("assistant_tool_call"),
  /** The tool call this is part of, when the loop ran it inside one; absent at the top. */
  parent_tool_call_id: z.string().optional(),
  /** This call's id, which its `ToolResponseChunk` echoes back. Ours, not MCP's: in MCP the JSON-RPC envelope correlates a request with its response, and a stream has no envelope. */
  id: z.string(),
  /** The request's `_meta`, MCP's own: a progress token, and what else the loop put there. */
  _meta: RequestMetaSchema.optional(),
  /** The name of the tool to call. */
  name: z.string(),
  /** One fragment of the call's arguments: JSON text, complete only once every fragment with this `id` has been concatenated. Matches the tool's input schema when whole. */
  arguments: z.string().optional(),
});
export type AssistantToolCallChunk = z.infer<typeof AssistantToolCallChunk>;
