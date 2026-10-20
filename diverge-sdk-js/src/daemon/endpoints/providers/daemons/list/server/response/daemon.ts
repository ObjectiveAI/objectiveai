// One daemon, as this daemon holds it.

import { z } from "zod";
import { Creator } from "../../../../../../../daemon/creator/creator.js";
import { Account } from "../../../../../../../daemon/endpoints/providers/daemons/account.js";
import { Link } from "../../../../../../../daemon/endpoints/providers/daemons/link.js";
import type { Keyed } from "../../../../../../../shared/listing/keyed.js";

/**
 * One daemon of the caller's: its name, its links, its accounts by
 * name and the kind of their mode, which of them this daemon holds a
 * connection as now, when it was added and by whom, and its tags.
 */
export const Daemon = z.object({
  /** The name, as the add gave it: the daemon's identity here. */
  name: z.string(),
  /** The providers it is reached through, each with the identity it is known by there. Absent when empty. */
  links: z.array(Link).optional(),
  /** The accounts this daemon holds on it, in the order they were added. Absent when empty. */
  accounts: z.array(Account).optional(),
  /** The accounts this daemon holds a connection as now, by name. Absent when none. */
  connected: z.array(z.string()).optional(),
  /** When the add made it: an RFC 3339 timestamp in UTC. */
  created: z.iso.datetime({ offset: true }),
  /** Who added it, the direct maker. */
  creator: Creator,
  /** The tags on it. Absent when empty. */
  tags: z.array(z.string()).optional(),
});
export type Daemon = z.infer<typeof Daemon>;

/** Keyed by the name. */
export const key: Keyed<Daemon> = (daemon) => daemon.name;
