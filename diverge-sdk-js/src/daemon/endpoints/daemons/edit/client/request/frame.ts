// A daemons edit request.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { DaemonId } from "../../../id.js";

/** Ask the daemon to change a record's description, by its ID: given, it replaces the record's; absent, it is left. Nothing else of a record changes: what it reaches and as whom is its ID, so reaching elsewhere is a delete and an add, with another ID. A request naming any other member does not parse. */
export const Frame = z.strictObject({
  /** The record, by its ID. */
  id: DaemonId,
  /** The description, in place of the record's. */
  description: z.string().optional(),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;


/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 131;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
