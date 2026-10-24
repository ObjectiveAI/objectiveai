// Creating a tool from a template.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Identity } from "../../../../../../shared/identity.js";
import { Inner } from "../../../../../../daemon/create/inner.js";

/**
 * Ask the daemon to make a tool: from a template by its id, under a
 * name if one is given, as an account of the daemon's if it is to
 * hold any grant, with the vault keys its secrets come from,
 * pinned to a provider with that provider's volumes mounted, and
 * with providers' volumes served live over FUSE — `create.Inner`,
 * flattened, the account and the name. Nothing runs at a create. A
 * name is unique among the caller's tools — a namespace of their
 * own, so a tool and an agent may share one — else `in_use`. Absent
 * an account, everything the tool asks the daemon is forbidden.
 */
export const Frame = z.object({
  ...Inner.shape,
  /** The account the tool acts as toward the daemon, by its identity, if any; absent, it holds no grant. */
  account: Identity.optional(),
  /** The name, if the tool is to have one; unique among the caller's tools. Absent, none. */
  name: z.string().optional(),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 15;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
