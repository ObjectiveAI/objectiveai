// Creating an agent from a template.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Inner } from "../../../../../../daemon/create/inner.js";

/**
 * Ask the daemon to make an agent: from a template by its id, under
 * a name if one is given, with the vault keys its secrets come from,
 * pinned to a provider with that provider's volumes mounted, and
 * with providers' volumes served live over FUSE — `create.Inner`,
 * flattened, and the name. Nothing runs: the container is work made
 * from the record on its first message. A name is unique among the
 * caller's agents, else `in_use`; a nameless agent is reached by
 * template and index.
 */
export const Frame = z.object({
  ...Inner.shape,
  /** The name, if the agent is to have one; unique among the caller's agents. Absent, none. */
  name: z.string().optional(),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 0;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
