// One provider, as the daemon reports it.

import { z } from "zod";
import { Creator } from "../../../../../creator/creator.js";
import { Direction } from "../../../direction.js";
import { Status } from "../../../status.js";
import { Frame as Capacity } from "../../../../../../provider/endpoints/capacity/server/response/frame.js";
import { Description } from "../../../../../../shared/description.js";
import { Identity } from "../../../../../../shared/identity.js";
import type { Keyed } from "../../../../../../shared/listing/keyed.js";

/**
 * One provider the daemon keeps a record of: its identity, the direction
 * its connection is made in — its members beside the record's own —
 * what the daemon announces itself with there, whether it accepts
 * other daemons and hears messages through it, whether it is enabled,
 * what it is in the caller's words and its tags, who made the record
 * and when, and where the daemon stands with it now — and, connected,
 * its latest capacity.
 */
export const Provider = z.intersection(
  z.object({
    /** The provider's identity: the `(issuer, subject)` its token establishes in the handshake. */
    identity: Identity,
    /** What the daemon announces itself with on it: the text of the `daemons::announce` the daemon holds open there while connected, by which its `daemons::list` lists the daemon. Absent, the daemon does not announce itself there. */
    announcement: Description.optional(),
    /** Whether the daemon accepts connections from other daemons through it: holds its `daemons::accept` open while connected. */
    accept_daemons: z.boolean(),
    /** Whether the daemon listens for messages on it: holds its `daemons::listen` open while connected. */
    messageable: z.boolean(),
    /** Whether the daemon works with it: dials it, or admits it. */
    enabled: z.boolean(),
    /** What it is, in the caller's words — the operator's note, which the provider never sees; empty where none was given. */
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
