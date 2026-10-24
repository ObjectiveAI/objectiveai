// Deleting a vault key, or one of its values.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";

/**
 * Ask the daemon to delete a vault key, by name, or one value of it,
 * by number. A key an agent or a tool binds is not deleted; nor is a
 * value a running container holds, nor a key with one; nor a key's
 * last value, which is the key itself.
 */
export const Frame = z.object({
  /** The key's name. */
  key: z.string(),
  /** Which of the key's values to delete, by number. Absent, the whole key with every value it holds. */
  slot: z.number().int().nonnegative().optional(),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 105;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
