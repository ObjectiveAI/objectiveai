// Editing an agent.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Agent } from "../../../../../../daemon/reference/agent.js";
import { Patch } from "../../../../../../daemon/patch/operation.js";

/**
 * Ask the daemon to change an agent: a JSON Patch over its
 * `Editable`, applied whole or not at all. A running agent takes
 * only FUSE mounts appended to either list, mounted into the running
 * container before the edit persists; any other difference is
 * `active`. The image, the limits, the provider and the arguments
 * are never editable; which vault key each secret is, is.
 */
export const Frame = z.object({
  /** The agent: by name, or by template and index. */
  agent: Agent,
  /** The operations, in order: see `daemon.patch`. Empty changes nothing. */
  patch: Patch,
  /** Served by another daemon, as one of the accounts this daemon holds on it. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 6;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
