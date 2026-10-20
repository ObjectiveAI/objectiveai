// One kind of chunk: `assistant_text_content`.

import { z } from "zod";
import { TextContentSchema } from "@modelcontextprotocol/core";
import { Logprob } from "./logprobs.js";

/**
 * Text from the model.
 *
 * A DELTA: text arrives in fragments, and a caller concatenates them.
 * Unlike the image and audio chunks, one of these is rarely a whole
 * anything.
 *
 * The payload is MCP's own `TextContent`, flattened, so the content the
 * model produced is expressed in the same vocabulary a tool would use
 * to return it — one content model across the whole loop rather than
 * one per direction.
 */
export const AssistantTextContentChunk = z.object({
  /** Which kind this is. */
  type: z.literal("assistant_text_content"),
  /** The tool call this is part of, when the loop ran it inside one; absent at the top. */
  parent_tool_call_id: z.string().optional(),
  /** What the model thought of each token, where the provider says. */
  logprobs: z.array(Logprob).optional(),
  /** The content, MCP's own, flattened in: its `type` is this chunk's. */
  ...TextContentSchema.omit({ type: true }).shape,
});
export type AssistantTextContentChunk = z.infer<typeof AssistantTextContentChunk>;
