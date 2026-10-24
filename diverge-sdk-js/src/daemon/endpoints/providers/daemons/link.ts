// One way a daemon is reached: a provider, and the daemon's identity there.

import { z } from "zod";
import { Identity } from "../../../../shared/identity.js";

/**
 * One link of a remote daemon: a provider the daemon's configuration
 * accepts, by its identity, and the identity the remote daemon is
 * known by at that provider. A remote's links are every provider it
 * is reachable through.
 */
export const Link = z.object({
  /** The provider, by its identity. */
  provider: Identity,
  /** The remote daemon, by its identity at that provider. */
  daemon: Identity,
});
export type Link = z.infer<typeof Link>;
