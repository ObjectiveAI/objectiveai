// Getting one remote daemon, by name.

import { z } from "zod";
import { json, tagged } from "../../../../../../../wire/encode.js";
import { Relay } from "../../../../../../../shared/relay.js";

/** Ask the daemon for one remote daemon of its configuration's, by name. */
export const Frame = z.object({
  /** The remote's name, as the configuration gives it. */
  name: z.string(),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 83;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
