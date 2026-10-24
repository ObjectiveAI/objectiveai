// A provider's identity.

import { z } from "zod";

/**
 * A provider's identity: one of the ways the daemon came to be
 * connected to it, and what that way establishes. One object, told
 * apart by `kind`: `outgoing`, the daemon dialled the provider and
 * the address is the identity; `incoming_unbrokered`, the provider
 * dialled the daemon and the identity is what the daemon's judging
 * of its credential answered. `incoming_brokered` is reserved.
 */
export const Identity = z.discriminatedUnion("kind", [
  z.object({
    kind: z.literal("outgoing"),
    /** Where the provider was dialled, as the daemon's configuration writes it: a URL, or a bare `host:port`. */
    address: z.string(),
  }),
  z.object({
    kind: z.literal("incoming_unbrokered"),
    /** The provider's identity, as the daemon's judging of its credential answered. */
    identity: z.string(),
  }),
]);
export type Identity = z.infer<typeof Identity>;
