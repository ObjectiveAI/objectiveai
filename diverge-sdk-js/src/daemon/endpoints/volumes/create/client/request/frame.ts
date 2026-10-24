// Creating a volume on a provider.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Identity } from "../../../../../../daemon/endpoints/agents/logs/server/response/identity.js";
import { Mode } from "../../../../../../provider/endpoints/volumes/mode.js";

/**
 * Ask the daemon to create a volume on a provider, as the provider
 * protocol's `volumes.create` takes one: a name of the caller's
 * choosing, how many bytes it reserves, and its mode. A name a
 * volume of that provider's has already is refused. The volume is
 * the provider's from then on, and outlives every container and
 * connection; only a delete ends it.
 */
export const Frame = z.object({
  /** The provider to make it on. */
  provider: Identity,
  /** What to call it: the name the provider lists it under from then on. */
  name: z.string(),
  /** How many bytes it reserves, to start with; an edit changes it later. */
  bytes: z.number().int().nonnegative(),
  /** The mode it starts in: `persistent`, `ephemeral` or `read_only`. */
  mode: Mode,
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 65;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
