// A daemons delete request.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";

/** Ask the daemon to delete a record, by its name. Refused while a connected tool names it. */
export const Frame = z.object({
  /** The record, by its name. */
  name: z.string(),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;


/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 132;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
