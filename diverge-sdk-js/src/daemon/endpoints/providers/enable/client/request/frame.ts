// A providers enable request.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Identity } from "../../../../../../shared/identity.js";
import { Relay } from "../../../../../../shared/relay.js";

/** Ask the daemon to work with a provider again, by its identity: dialled again when outgoing, admitted again when incoming. */
export const Frame = z.object({
  /** The provider, by the identity its token establishes. */
  identity: Identity,
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 139;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
