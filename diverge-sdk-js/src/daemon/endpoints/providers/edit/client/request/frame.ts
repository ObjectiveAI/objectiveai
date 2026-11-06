// A providers edit request.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Direction } from "../../../direction.js";
import { Identity } from "../../../../../../shared/identity.js";
import { Relay } from "../../../../../../shared/relay.js";

/** Ask the daemon to change a provider's record, by its identity: every member given replaces the record's; a direction given replaces the whole of it. A change to the direction or the discover flag closes the connection. */
const Base = z.object({
  /** The provider, by the identity its token establishes. */
  identity: Identity,
  /** The discover flag, in place of the record's. */
  discover: z.boolean().optional(),
  /** The description, in place of the record's. */
  description: z.string().optional(),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});

export const Frame = z.union([z.intersection(Base, Direction), Base]);
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 137;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
