// One outgoing provider, as the daemon holds it.

import { z } from "zod";
import { Creator } from "../../../../../../../daemon/creator/creator.js";
import { Status } from "../../../../../../../daemon/endpoints/providers/status.js";
import { Kind } from "../../../../../../../daemon/endpoints/providers/outgoing/kind.js";
import { Frame as Capacity } from "../../../../../../../provider/endpoints/capacity/server/response/frame.js";
import type { Keyed } from "../../../../../../../shared/listing/keyed.js";

/**
 * One outgoing provider of the caller's: its address, which is its
 * identity, the kind of its mode without the credential, where the
 * daemon stands with it now, the provider's latest capacity answer,
 * when a connection last opened or closed, when it was added and by
 * whom, and its tags.
 */
export const Outgoing = z.object({
  /** The address, as the add gave it: the provider's identity. */
  address: z.string(),
  /** Which mode it is dialled in, without the credential. */
  kind: Kind,
  /** Where the daemon stands with it now. */
  status: Status,
  /** The provider's latest answer to the daemon's `capacity` ask. Absent until it has answered once, and whenever the status is not `connected`. */
  capacity: Capacity.optional(),
  /** When the connection last opened, if one is held, or last closed; absent for a provider never dialled. */
  last_connected: z.iso.datetime({ offset: true }).optional(),
  /** When the add made it: an RFC 3339 timestamp in UTC. */
  created: z.iso.datetime({ offset: true }),
  /** Who added it, the direct maker. */
  creator: Creator,
  /** The tags on it. Absent when empty. */
  tags: z.array(z.string()).optional(),
});
export type Outgoing = z.infer<typeof Outgoing>;

/** Keyed by the address: the provider's identity. */
export const key: Keyed<Outgoing> = (outgoing) => outgoing.address;
