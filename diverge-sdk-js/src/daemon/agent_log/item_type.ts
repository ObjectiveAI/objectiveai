// The kinds of item a log holds.

import { z } from "zod";

/**
 * One kind of item, as a log request narrows to it: the sixteen
 * chunk kinds by their `type`, the error, and the four steps of a
 * run's life — `deployed`, `active`, `inactive`, `undeployed`.
 */
export const ItemType = z.enum([
  "user_text_content",
  "user_image_content",
  "user_audio_content",
  "user_resource",
  "user_resource_link",
  "user_structured_decisions",
  "structured_decisions_response",
  "assistant_reasoning",
  "assistant_text_content",
  "assistant_image_content",
  "assistant_audio_content",
  "assistant_tool_call",
  "assistant_refusal",
  "tool_response",
  "usage",
  "notification",
  "error",
  "deployed",
  "active",
  "inactive",
  "undeployed",
]);
export type ItemType = z.infer<typeof ItemType>;
