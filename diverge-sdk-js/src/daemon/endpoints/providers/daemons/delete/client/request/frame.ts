// Forgetting a daemon.

import { z } from "zod";
import { json, tagged } from "../../../../../../../wire/encode.js";
import { Relay } from "../../../../../../../shared/relay.js";

/**
 * Ask the daemon to forget a daemon record, by name. One a connected
 * tool names is not forgotten, and the daemon says so with a variant
 * of its own: delete the tools, and ask again.
 */
export const Frame = z.object({
  /** The daemon's name, as its add gave it. */
  name: z.string(),
  /** Served by another daemon, as one of the accounts this daemon holds on it. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 85;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
