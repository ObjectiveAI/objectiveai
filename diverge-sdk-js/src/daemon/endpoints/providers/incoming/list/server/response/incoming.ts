// One credential of incoming providers, as the daemon holds it.

import { z } from "zod";
import { Creator } from "../../../../../../../daemon/creator/creator.js";
import { Status } from "../../../../../../../daemon/endpoints/providers/status.js";
import { Credential } from "../../../../../../../daemon/endpoints/providers/incoming/credential.js";
import { Frame as Capacity } from "../../../../../../../provider/endpoints/capacity/server/response/frame.js";
import type { Keyed } from "../../../../../../../shared/listing/keyed.js";

/**
 * One credential: the identity it names and the address it is
 * accepted from — never the key — flat among the members; where the
 * daemon stands with the provider connected through it now; the
 * provider's latest capacity answer; when it was added and by whom;
 * and its tags. A list sends them oldest added first.
 */
export const Incoming = z.object({
  ...Credential.shape,
  /** Where the daemon stands with the provider connected through it now: exactly one is, or none. */
  status: Status,
  /** The connected provider's latest answer to the daemon's `capacity` ask. Absent until answered, and whenever not `connected`. */
  capacity: Capacity.optional(),
  /** When the add made it: an RFC 3339 timestamp in UTC. */
  created: z.iso.datetime({ offset: true }),
  /** Who added it, the direct maker. */
  creator: Creator,
  /** The tags on it. Absent when empty. */
  tags: z.array(z.string()).optional(),
});
export type Incoming = z.infer<typeof Incoming>;

/** Keyed by the credential's identity. */
export const key: Keyed<Incoming> = (incoming) => incoming.identity;
