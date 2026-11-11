// Listing peers through the connected providers, and keeping the
// listing.

import { z } from "zod";
import { json, tagged } from "../../../../../../../wire/encode.js";
import { Relay } from "../../../../../../../shared/relay.js";

/**
 * Ask the daemon for every identity announced on its connected
 * providers, each with the providers it was seen on. Everything is
 * optional, and a request with none of it — `{}` on the wire — is
 * every one.
 */
export const Frame = z.object({
  /** How many the listing is kept to: the first that many. `0` is a listing of nothing. Absent, no cap. */
  count: z.number().int().nonnegative().optional(),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 125;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
