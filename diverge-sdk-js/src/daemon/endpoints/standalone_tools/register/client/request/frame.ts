// Registering another daemon's tool under a name.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Tool } from "../../../../../../daemon/reference/tool.js";
import { DaemonId } from "../../../../daemons/id.js";

/**
 * Ask the daemon to keep another daemon's tool as one of the
 * caller's: the remote daemon, by the ID of this daemon's record of it — the
 * caller holding the daemons grant's `connect` — and the tool as that
 * daemon names it, under a name if one is given. Nothing is
 * connected now: when an attached agent uses the tool, the daemon
 * opens the far daemon's `connect` and drops it when idle. A daemon
 * that cannot be reached is not this request's error; it shows as
 * the tool inactive, and as the agent's calls failing.
 */
export const Frame = z.object({
  /** The remote daemon the tool is on, by the ID of this daemon's record of it: see `daemons`. An ID no record has is the register's error. */
  daemon: DaemonId,
  /** The remote-daemon records the tool's connect is relayed through beyond `daemon`, each by its ID on the daemon before it: the tool is on the last. Each hop is served as the account the previous daemon's record names, and no hop can be re-pointed, an ID being the hash of what its record reaches. Absent or empty, the tool is on `daemon`. */
  route: z.array(DaemonId).optional(),
  /** The tool, as that daemon names it: by name, by template and index, or another daemon's by the daemon and its reference there. */
  tool: Tool,
  /** The name, if the tool is to have one; unique among the caller's tools. Absent, none. */
  name: z.string().optional(),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 18;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
