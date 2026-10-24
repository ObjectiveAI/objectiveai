// Listing the remote daemons, narrowed, and keeping the list.

import { z } from "zod";
import { json, tagged } from "../../../../../../../wire/encode.js";
import { Relay } from "../../../../../../../shared/relay.js";
import { Filter } from "./filter.js";

/**
 * Ask the daemon for the remote daemons of its configuration's,
 * narrowed. Everything is optional, and a request with none of it —
 * `{}` on the wire — is every one. The members but `count` are the
 * filter, flattened in.
 */
export const Frame = z.object({
  ...Filter.shape,
  /** How many the list is kept to: the first that many that match. `0` is a list of nothing. Absent, no cap. */
  count: z.number().int().nonnegative().optional(),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 84;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
