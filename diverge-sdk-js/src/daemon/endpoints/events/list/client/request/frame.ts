// Listing events, narrowed, and keeping the list.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Filter } from "./filter.js";

/**
 * Ask the daemon for the events, narrowed. Everything is optional,
 * and a request with none of it — `{}` on the wire — is every event.
 * The members but `count` are the filter, flattened in; `count`
 * keeps the initial listing to the LATEST that many by id, and what
 * follows the word that the list is whole has no cap.
 */
export const Frame = z.object({
  ...Filter.shape,
  /** How many events the initial listing is kept to: the latest that many by id. `0` is a listing of nothing. Absent, every event that matches. */
  count: z.number().int().nonnegative().optional(),
  /** Served by another daemon, as one of the accounts this daemon holds on it. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 109;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
