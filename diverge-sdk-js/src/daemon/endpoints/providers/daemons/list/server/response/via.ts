// One route to a listed identity: a provider that lists it, and where.

import { z } from "zod";
import { Identity } from "../../../../../../../shared/identity.js";

/**
 * One provider whose `daemons.list` listing holds the identity, with where
 * that provider holds its connection: the IP of the identity's
 * connection to it, and the dial target when the provider dialled it.
 */
export const Via = z.object({
  /** The provider, by its identity. */
  provider: Identity,
  /** The other end of the identity's connection to the provider. */
  ip: z.string(),
  /** The dial target the provider dialled it at — a `host:port` or a URL — when it did; absent when it dialled the provider. */
  address: z.string().optional(),
  /** Whether the identity listens for messages through the provider. */
  listening: z.boolean(),
});
export type Via = z.infer<typeof Via>;
