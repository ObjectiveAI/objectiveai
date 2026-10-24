// Forgetting an account on a daemon record.

import { z } from "zod";
import { json, tagged } from "../../../../../../../../wire/encode.js";
import { Relay } from "../../../../../../../../shared/relay.js";

/**
 * Ask the daemon to forget an account on a daemon record. An account
 * a connected tool names is refused: the tool is deleted first.
 */
export const Frame = z.object({
  /** The daemon's name, as its add gave it. */
  daemon: z.string(),
  /** The account's name on it. */
  name: z.string(),
  /** Served by another daemon, as one of the accounts this daemon holds on it. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 100;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
