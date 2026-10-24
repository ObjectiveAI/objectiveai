// Naming a provider to dial.

import { z } from "zod";
import { json, tagged } from "../../../../../../../wire/encode.js";
import { Relay } from "../../../../../../../shared/relay.js";
import { Mode } from "../../../../../../../daemon/endpoints/providers/outgoing/mode.js";

/**
 * Ask the daemon to add an outgoing provider: this address, dialled
 * in this mode. The address is the identity — the string as given, a
 * URL or a `host:port` — and one the caller has added already is
 * refused. From then on the daemon dials the address, in that mode,
 * for whatever it needs of the provider.
 */
export const Frame = z.object({
  /** Where to dial, as the daemon is to write it: the provider's identity from then on. */
  address: z.string(),
  /** How to authenticate there, by the mode's name. */
  mode: Mode,
  /** Served by another daemon, as one of the accounts this daemon holds on it. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 31;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
