// Inserting one value into a vault key.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";

/**
 * Ask the daemon to insert a value into a vault key, by name: make the
 * key, where the vault holds no key of the name, the value its slot
 * `0`; or, where it does, append the value as one more slot, or
 * replace the value of the slot named. A key some agent or tool binds
 * is inserted into like any other: the next activation may be handed
 * the new value.
 */
export const Frame = z.object({
  /** The key's name. */
  key: z.string(),
  /** Which of the key's values to replace, by number. Absent, the value is appended as a new slot — the first, where the key is new. */
  slot: z.number().int().nonnegative().optional(),
  /** The value: any JSON, held as given and handed to a container that takes the key. */
  value: z.json(),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 104;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
