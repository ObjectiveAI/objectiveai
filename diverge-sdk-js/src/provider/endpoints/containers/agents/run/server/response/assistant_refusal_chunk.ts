// One kind of chunk: `assistant_refusal`.

import { z } from "zod";
import { TextContentSchema } from "@modelcontextprotocol/core";
import { Logprob } from "./logprobs.js";

/**
 * The model declining to answer.
 *
 * A DELTA, like `AssistantTextContentChunk`: fragments arrive and a
 * caller concatenates them.
 *
 * The payload is MCP's `TextContent`, not a bare `String`. Same
 * vocabulary as ordinary content, so nothing here needs its own
 * handling — and `_meta` and `annotations` come along, which a `String`
 * has nowhere to put.
 *
 * Structurally identical to the text and reasoning chunks, and that is
 * fine precisely because the `type` constants differ: the untagged enum
 * decides on the discriminator, never on shape, so payloads may
 * coincide without becoming ambiguous.
 */
export const AssistantRefusalChunk = z.object({
  /** Which kind this is. */
  type: z.literal("assistant_refusal"),
  /** The tool call this is part of, when the loop ran it inside one; absent at the top. */
  parent_tool_call_id: z.string().optional(),
  /** What the model thought of each token, where the provider says. */
  logprobs: z.array(Logprob).optional(),
  /** The content, MCP's own, flattened in: its `type` is this chunk's. */
  ...TextContentSchema.omit({ type: true }).shape,
});
export type AssistantRefusalChunk = z.infer<typeof AssistantRefusalChunk>;
