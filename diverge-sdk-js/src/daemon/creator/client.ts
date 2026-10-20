// The client as a creator.

import { z } from "zod";

/**
 * The client itself: what made a thing over an endpoint, and what
 * every agent and tool of the client's descends from, one maker at
 * a time.
 */
export const Client = z.object({
  /** The account the create was served for: its name when it has one, else the identity its credential names. */
  identity: z.string(),
});
export type Client = z.infer<typeof Client>;
