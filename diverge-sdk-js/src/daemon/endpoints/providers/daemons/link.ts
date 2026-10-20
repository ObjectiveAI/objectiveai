// One way a daemon is reached: a provider, and its name for the daemon.

import { z } from "zod";
import { Identity } from "../../agents/logs/server/response/identity.js";

/**
 * One link of a daemon record: a provider of the caller's, by the
 * identity this daemon knows it by, and the identity the remote
 * daemon is known by at that provider. A daemon's links are every
 * provider it is reachable through.
 */
export const Link = z.object({
  /** The provider, as this daemon names one: one of the caller's, outgoing or incoming, on record. */
  provider: Identity,
  /** The remote daemon's identity at that provider. */
  identity: z.string(),
});
export type Link = z.infer<typeof Link>;
