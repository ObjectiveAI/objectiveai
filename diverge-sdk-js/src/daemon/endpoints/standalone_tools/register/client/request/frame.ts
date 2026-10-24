// Registering another daemon's tool under a name.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Tool } from "../../../../../../daemon/reference/tool.js";

/**
 * Ask the daemon to keep another daemon's tool as one of the
 * caller's: the `providers.daemons` record, the account held there —
 * over which the caller holds the `connect` grant — and the tool as
 * that daemon names it, under a name if one is given. Nothing is
 * connected now: when an attached agent uses the tool, the daemon
 * opens the far daemon's `connect` and drops it when idle. A daemon
 * that cannot be reached is not this request's error; it shows as
 * the tool inactive, and as the agent's calls failing.
 */
export const Frame = z.object({
  /** The `providers.daemons` record, by name. */
  daemon: z.string(),
  /** The account held on that daemon, by its name in the record. */
  account: z.string(),
  /** The tool: by name, by template and index, or another daemon's by the daemon, the account and its reference there. */
  tool: Tool,
  /** The name, if the tool is to have one; unique among the caller's tools. Absent, none. */
  name: z.string().optional(),
  /** Served by another daemon, as one of the accounts this daemon holds on it. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 18;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
