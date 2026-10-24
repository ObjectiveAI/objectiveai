// Editing a tool.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Tool } from "../../../../../../daemon/reference/tool.js";
import { Patch } from "../../../../../../daemon/patch/operation.js";

/**
 * Ask the daemon to change a tool: a JSON Patch over its `Editable`,
 * applied whole or not at all. A running container takes only FUSE
 * mounts appended to either list, mounted live before the edit
 * persists; any other difference is `active`. A connected tool's
 * document is its name and nothing else: anything but the name
 * differing is `not_owned`. Empty changes nothing.
 */
export const Frame = z.object({
  /** The tool: by name, by template and index, or another daemon's by the daemon, the account and its reference there. */
  tool: Tool,
  /** The operations, in order: see `daemon.patch`. Empty changes nothing. */
  patch: Patch,
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 17;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
