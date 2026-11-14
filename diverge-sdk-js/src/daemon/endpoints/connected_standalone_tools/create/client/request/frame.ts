// Holding another daemon's standalone tool under a name.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Tool } from "../../../../../../daemon/reference/tool.js";
import { DaemonId } from "../../../../daemons/id.js";

/**
 * Ask the daemon to hold another daemon's standalone tool as one of
 * the caller's connected tools: the remote daemon, by the ID of this
 * daemon's record of it — the caller holding the daemons grant's
 * `relay` — and the tool as that daemon names it, under a name if
 * one is given. Nothing is connected now: when an attached agent uses
 * the tool, the daemon opens the far daemon's `standalone_tools`
 * `connect` and lets it go when idle. A daemon that cannot be
 * reached is not this request's error; it shows as the tool not
 * deployed, and as the agent's calls failing.
 */
export const Frame = z.object({
  /** The remote daemon the tool is on, by the ID of this daemon's record of it: see `daemons`. An ID no record has is the create's error. */
  daemon: DaemonId,
  /** The remote-daemon records the tool's connect is relayed through beyond `daemon`, each by its ID on the daemon before it: the tool is on the last. Absent when empty, and then the tool is on `daemon`. */
  route: z.array(DaemonId).optional(),
  /** The tool, as that daemon names it: by its name there, or by its template and its index there. */
  tool: Tool,
  /** The name, if the tool is to have one; unique among the caller's connected tools. Absent, none. */
  name: z.string().optional(),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 146;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
