// Asking the daemon what it is.

import { z } from "zod";
import { json, tagged } from "../../../../../wire/encode.js";
import { Relay } from "../../../../../shared/relay.js";

/**
 * Ask the daemon what it is. Carries nothing of its own, as the
 * provider's info does; only the `relay` every daemon request carries,
 * to ask another daemon instead.
 */
export const Frame = z.object({
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 111;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
