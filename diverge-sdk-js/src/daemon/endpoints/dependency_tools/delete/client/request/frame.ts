// Deleting a dependency template, and every tool of it.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";

/**
 * Ask the daemon to forget a dependency template, by id — the hash
 * each tool's list item carries, compared and not read — with every
 * tool of it under every owner, each tool's database scope and what
 * it held. A template any tool of which runs is `in_use` and stays:
 * wait for the runs to end, and ask again. An agent whose last
 * registration declared the template depends on it again when it
 * registers and declares it, which makes its tool anew.
 */
export const Frame = z.object({
  /** The dependency template, by id. */
  id: z.string(),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 91;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
