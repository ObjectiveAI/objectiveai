// The kinds of item a log holds, by name, across every book.

import { z } from "zod";

/**
 * One kind of item, by the name its `type` carries: every name an
 * agent's log item or a tool's log item has. A name both books have —
 * `deployed`, `active`, `inactive`, `undeployed`, `error`,
 * `notification` — picks that kind in
 * whichever book is read; a name the book read does not have matches
 * nothing.
 */
export const ItemType = z.enum([
  "user_text_content",
  "user_image_content",
  "user_audio_content",
  "user_resource",
  "user_resource_link",
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
  "call",
  "call_result",
  "call_error",
  "call_interrupted",
  "read",
  "read_result",
  "read_error",
  "read_interrupted",
  "command",
]);
export type ItemType = z.infer<typeof ItemType>;
