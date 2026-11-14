// Getting one of the host's identities.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Identity } from "../../../../../../shared/identity.js";

/** Ask the daemon for one of the host's identities, by its issuer and its subject. */
export const Frame = z.object({
  /** The identity: its issuer and its subject, as `list` answers them. */
  identity: Identity,
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 161;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
