// The kinds of item a log holds.

import { z } from "zod";

/**
 * One kind of item, as a log request narrows to it: the fourteen
 * chunk kinds by their `type`, the error, and the two marks of a
 * run — `active` when a container came up, `inactive` when it went.
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
  "active",
  "inactive",
]);
export type ItemType = z.infer<typeof ItemType>;
