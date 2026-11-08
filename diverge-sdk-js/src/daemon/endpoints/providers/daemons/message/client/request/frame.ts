// Sending one direct message through the daemon's providers.

import { z } from "zod";
import { json, tagged } from "../../../../../../../wire/encode.js";
import { Identity } from "../../../../../../../shared/identity.js";
import { Relay } from "../../../../../../../shared/relay.js";

/**
 * Send one message to `to`, through whichever of the daemon's providers
 * it listens on. JSON, as every request of the daemon's is: the body is
 * standard base64.
 */
export const Frame = z.object({
  /** The identity to message, as discovery lists it. */
  to: Identity,
  /** The body, as standard base64. Neither the daemon nor a provider reads it. */
  body: z.base64(),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 144;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
