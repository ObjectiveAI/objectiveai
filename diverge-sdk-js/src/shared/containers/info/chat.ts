// What an agent that chats says of itself.

import { z } from "zod";

import { Modalities } from "./modalities.js";

/**
 * How an agent's program chats: one member, required, and no other.
 * Stated by an agent's image, in the frontmatter's `chat`. Nothing
 * reads it yet.
 */
export const Chat = z.strictObject({
  /** What a message to it may carry beside text. */
  prompt_modalities: Modalities,
});
export type Chat = z.infer<typeof Chat>;
