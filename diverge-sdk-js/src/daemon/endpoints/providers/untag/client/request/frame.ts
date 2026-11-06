// A providers untag request.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Identity } from "../../../../../../shared/identity.js";
import { Relay } from "../../../../../../shared/relay.js";

/** Ask the daemon to take tags off a provider's record, by its identity. */
export const Frame = z.object({
  /** The provider, by the identity its token establishes. */
  identity: Identity,
  /** The tags to take off, one by one. */
  tags: z.array(z.string()),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 142;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
