// Attaching a tool to every agent of a template.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Tool } from "../../../../../../daemon/reference/tool.js";

/**
 * Ask the daemon to attach a tool to every agent made from an agent template,
 * now and later. Attaching one attached already changes nothing.
 */
export const Frame = z.object({
  /** The tool: by name, by template and index, or another daemon's by the daemon, the account and its reference there. */
  tool: Tool,
  /** The agent template, by id. */
  template: z.string(),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 101;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
