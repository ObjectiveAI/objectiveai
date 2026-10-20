// One kind of chunk: `assistant_audio_content`.

import { z } from "zod";
import { AudioContentSchema } from "@modelcontextprotocol/core";

/**
 * Audio from the model.
 *
 * Whole, not a delta — as with images, the payload is base64 data with
 * a MIME type.
 *
 * The payload is MCP's own `AudioContent`, flattened, so the content
 * the model produced is expressed in the same vocabulary a tool would
 * use to return it — one content model across the whole loop rather
 * than one per direction.
 */
export const AssistantAudioContentChunk = z.object({
  /** Which kind this is. */
  type: z.literal("assistant_audio_content"),
  /** The tool call this is part of, when the loop ran it inside one; absent at the top. */
  parent_tool_call_id: z.string().optional(),
  /** The content, MCP's own, flattened in: its `type` is this chunk's. */
  ...AudioContentSchema.omit({ type: true }).shape,
});
export type AssistantAudioContentChunk = z.infer<typeof AssistantAudioContentChunk>;
