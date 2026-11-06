// A providers add request.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Direction } from "../../../direction.js";
import { Identity } from "../../../../../../shared/identity.js";
import { Relay } from "../../../../../../shared/relay.js";

/** Ask the daemon to keep a provider's record, and to work with it from now on: dial it when outgoing, admit it when incoming. */
export const Frame = z.intersection(
  z.object({
  /** The provider, by the identity its token establishes. */
  identity: Identity,
  /** Whether the daemon is discoverable to the provider. Absent, `false`. */
  discover: z.boolean().optional(),
  /** Whether the daemon works with it from now on. Absent, `true`. */
  enabled: z.boolean().optional(),
  /** What it is, in the caller's words. Absent, none. */
  description: z.string().optional(),
  /** The tags to put on it. Absent, none. */
  tags: z.array(z.string()).optional(),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
  }),
  Direction,
);
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 136;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
