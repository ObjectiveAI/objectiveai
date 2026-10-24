// One remote daemon, as this daemon reports it.

import { z } from "zod";
import { Link } from "../../../../../../../daemon/endpoints/providers/daemons/link.js";
import type { Keyed } from "../../../../../../../shared/listing/keyed.js";

/**
 * One remote daemon of the configuration's: its name, the issuer this
 * daemon presents as to it, its links, and whether this daemon holds
 * a connection to it now.
 */
export const Daemon = z.object({
  /** The name, as the configuration gives it. */
  name: z.string(),
  /** The issuer this daemon presents as to it: the remote holds this daemon's identity there as one of its accounts. */
  issuer: z.string(),
  /** The providers it is reached through, each with the identity it is known by there. Absent when empty. */
  links: z.array(Link).optional(),
  /** Whether this daemon holds a connection to it now. */
  connected: z.boolean(),
});
export type Daemon = z.infer<typeof Daemon>;

/** Keyed by the name. */
export const key: Keyed<Daemon> = (daemon) => daemon.name;
