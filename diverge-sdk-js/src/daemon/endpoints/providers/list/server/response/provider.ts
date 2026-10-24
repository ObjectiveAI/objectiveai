// One provider, as the daemon reports it.

import { z } from "zod";
import { Status } from "../../../../../../daemon/endpoints/providers/status.js";
import { Frame as Capacity } from "../../../../../../provider/endpoints/capacity/server/response/frame.js";
import { Identity } from "../../../../../../shared/identity.js";
import type { Keyed } from "../../../../../../shared/listing/keyed.js";

/**
 * One provider the daemon accepts: its identity, the address the
 * daemon dials it at when its configuration dials it, and where the
 * daemon stands with it now — and, connected, what the provider
 * answered its `info` ask.
 */
export const Provider = z.object({
  /** The provider's identity: the `(issuer, subject)` its token establishes in the handshake. */
  identity: Identity,
  /** The address the daemon dials it at, as its configuration writes it; absent for a provider the daemon only accepts. */
  address: z.string().optional(),
  /** Where the daemon stands with it now. */
  status: Status,
  /** The provider's latest answer to the daemon's `capacity` ask. Absent until it has answered once, and whenever the status is not `connected`. */
  capacity: Capacity.optional(),
});
export type Provider = z.infer<typeof Provider>;

/** Keyed by the identity. */
export const key: Keyed<Provider> = (provider) => provider.identity;
