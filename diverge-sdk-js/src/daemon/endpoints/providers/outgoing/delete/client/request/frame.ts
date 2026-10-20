// Forgetting an outgoing provider.

import { z } from "zod";
import { json, tagged } from "../../../../../../../wire/encode.js";
import { Relay } from "../../../../../../../shared/relay.js";

/**
 * Ask the daemon to forget an outgoing provider, by address. A
 * provider in use — a container of the caller's pinned to it, or
 * mounting or serving a volume of its — is not forgotten, and the
 * daemon says so with a variant of its own.
 */
export const Frame = z.object({
  /** The address: the provider's identity. */
  address: z.string(),
  /** Served by another daemon, as one of the accounts this daemon holds on it. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 34;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
