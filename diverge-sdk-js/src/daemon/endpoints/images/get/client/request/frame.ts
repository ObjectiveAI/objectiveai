// Getting one image in a Store of the daemon's.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Identity } from "../../../../../agent_log/identity.js";
import { Relay } from "../../../../../../shared/relay.js";

/**
 * Ask the daemon for one image in its Store on a provider, by the
 * provider and the digest: the two that name it, as `list` answers
 * them.
 */
export const Frame = z.object({
  /** The provider whose Store of the daemon's is read. */
  provider: Identity,
  /** The image's manifest digest, `<algorithm>:<hex>`. */
  digest: z.string(),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 162;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
