// How much a provider relays between the identities connected to it.

import { z } from "zod";

/**
 * The rate at which a provider relays messages: at most `messages`
 * sends per sending identity in a fixed window of `seconds`. Both are
 * required and at least `1`: there is no unlimited provider, and a
 * provider that relays none states no object at all.
 */
export const MessagesCapabilities = z.strictObject({
  /** The most sends one identity makes in one window. */
  messages: z.number().int().min(1).max(4294967295),
  /** How long one window is, in seconds. */
  seconds: z.number().int().min(1).max(4294967295),
});
export type MessagesCapabilities = z.infer<typeof MessagesCapabilities>;
