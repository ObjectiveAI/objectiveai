// One kind of chunk: `assistant_image_content`.

import { z } from "zod";
import { ImageContentSchema } from "@modelcontextprotocol/core";

/**
 * An image from the model.
 *
 * Whole, not a delta — the payload is base64 data with a MIME type, and
 * half of a base64 image is not an image.
 *
 * The payload is MCP's own `ImageContent`, flattened, so the content
 * the model produced is expressed in the same vocabulary a tool would
 * use to return it — one content model across the whole loop rather
 * than one per direction.
 */
export const AssistantImageContentChunk = z.object({
  /** Which kind this is. */
  type: z.literal("assistant_image_content"),
  /** The tool call this is part of, when the loop ran it inside one; absent at the top. */
  parent_tool_call_id: z.string().optional(),
  /** The content, MCP's own, flattened in: its `type` is this chunk's. */
  ...ImageContentSchema.omit({ type: true }).shape,
});
export type AssistantImageContentChunk = z.infer<typeof AssistantImageContentChunk>;
