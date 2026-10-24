// Creating a tool template.

import { z } from "zod";
import { json, tagged } from "../../../../../../../wire/encode.js";
import { Relay } from "../../../../../../../shared/relay.js";
import { Template } from "../../../../../../../daemon/endpoints/standalone_tools/templates/template.js";

/**
 * Ask the daemon to keep a tool template: the template itself,
 * flattened — `{"type":"tool"}` and its `mode` among its members,
 * the image, the limits, the arguments, the secrets it declares —
 * whose id is its hash, so a template made twice is the one
 * template, answered `exists` with the id the caller already has.
 */
export const Frame = z.object({
  ...Template.shape,
  /** Served by another daemon, as one of the accounts this daemon holds on it. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 25;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
