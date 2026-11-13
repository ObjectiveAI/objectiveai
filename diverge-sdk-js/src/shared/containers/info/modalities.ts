// Which kinds of content, beside text, an agent takes.

import { z } from "zod";

/**
 * The kinds of content an agent's program takes beside text, each
 * said outright: three members, all required, and no other. Text is
 * not among them, since every agent takes it, nor a resource link,
 * which an agent is handed as text.
 */
export const Modalities = z.strictObject({
  /** Whether it takes an image: an MCP image block. */
  image: z.boolean(),
  /** Whether it takes audio: an MCP audio block. */
  audio: z.boolean(),
  /**
   * Whether it takes a file: an MCP embedded resource whose contents
   * are bytes and are neither an image nor audio.
   */
  file: z.boolean(),
});
export type Modalities = z.infer<typeof Modalities>;
