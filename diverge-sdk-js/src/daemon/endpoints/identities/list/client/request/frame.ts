// Listing the host's identities, each with its tags.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Filter } from "./filter.js";

/**
 * Ask the daemon for the identities signed in on its host, narrowed.
 * Everything is optional, and a request with none of it — `{}` on the
 * wire — is every identity the grants reach. The members but `relay`
 * are the filter, flattened in.
 */
export const Frame = z.object({
  ...Filter.shape,
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 126;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
