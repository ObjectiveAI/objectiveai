// One provider, as the daemon reports it.

import { z } from "zod";
import { Creator } from "../../../../../creator/creator.js";
import { Direction } from "../../../direction.js";
import { Status } from "../../../status.js";
import { Frame as Capacity } from "../../../../../../provider/endpoints/capacity/server/response/frame.js";
import { Identity } from "../../../../../../shared/identity.js";
import type { Keyed } from "../../../../../../shared/listing/keyed.js";

/**
 * One provider the daemon keeps a record of: its identity, the direction
 * its connection is made in — its members beside the record's own —
 * whether the daemon is listable to it and whether it is enabled,
 * its description and tags, who made the record and when, and where
 * the daemon stands with it now — and, connected, its latest capacity.
 */
export const Provider = z.intersection(
  z.object({
    /** The provider's identity: the `(issuer, subject)` its token establishes in the handshake. */
    identity: Identity,
    /** Whether the daemon is listable to it: listed by its `daemons::list`. */
    listable: z.boolean(),
    /** Whether the daemon works with it: dials it, or admits it. */
    enabled: z.boolean(),
    /** What it is, in the caller's words; empty where none was given. */
    description: z.string(),
    /** The tags on it, sorted. */
    tags: z.array(z.string()),
    /** When the record was made. */
    created: z.string(),
    /** Who made it. */
    creator: Creator,
    /** Where the daemon stands with it now. */
    status: Status,
    /** The provider's latest answer to the daemon's `capacity` ask. Absent until it has answered once, and whenever the status is not `connected`. */
    capacity: Capacity.optional(),
  }),
  Direction,
);
export type Provider = z.infer<typeof Provider>;

/** Keyed by the identity. */
export const key: Keyed<Provider> = (provider) => provider.identity;
