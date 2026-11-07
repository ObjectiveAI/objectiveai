// One remote daemon's record, as this daemon reports it.

import { z } from "zod";
import { Creator } from "../../../../../creator/creator.js";
import { DaemonId } from "../../../id.js";
import { Routes } from "../../../routes.js";
import { Identity } from "../../../../../../shared/identity.js";
import type { Keyed } from "../../../../../../shared/listing/keyed.js";

/**
 * One remote daemon this daemon reaches: a record of one account of ours
 * there. The ID is how relays and connected tools name it: the hash of
 * `daemon`, `identity` and the routes, none of which ever changes; only
 * the description and the tags do. `daemon` is the remote, by the identity it answers as, and several records may
 * name the same remote; `identity` is ours, presented to it, at the
 * remote's issuer. It is reached through any connected provider whose
 * discover listing holds `daemon` by a route its `Routes` let through.
 */
export const Daemon = z.object({
  /** The ID: what it reaches and as whom, hashed. Unique among the records. */
  id: DaemonId,
  ...Routes.shape,
  /** What it is, in the caller's words; empty where none was given. */
  description: z.string(),
  /** The remote daemon, by the identity it answers as. */
  daemon: Identity,
  /** The identity this daemon presents to it: our account there. */
  identity: Identity,
  /** The tags on it, sorted. */
  tags: z.array(z.string()),
  /** When it was added. */
  created: z.string(),
  /** Who added it. */
  creator: Creator,
  /** Whether this daemon holds a connection to it now. */
  connected: z.boolean(),
});
export type Daemon = z.infer<typeof Daemon>;

/** Keyed by the ID. */
export const key: Keyed<Daemon> = (daemon) => daemon.id;
