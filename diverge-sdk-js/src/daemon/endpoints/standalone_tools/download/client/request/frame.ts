// Downloading a file or a directory out of a tool's container.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Tool } from "../../../../../../daemon/reference/tool.js";

/**
 * Ask the daemon to send a file or a directory out of a tool's
 * container. What comes back is addressed relative to the path
 * named: a file's chunks carry the empty path, a directory's each
 * file's path within it — see `daemon.download.Chunk`. A container
 * that is not running is started for the operation and stopped when
 * it finishes.
 */
export const Frame = z.object({
  /** The tool: by name, by template and index, or another daemon's by the daemon, the account and its reference there. */
  tool: Tool,
  /** The file or directory, as components from the container's root: empty, the whole root. Each component a name: not empty, not `.` or `..`, holding no `/` and no NUL. */
  path: z.array(z.string()),
  /** Served by another daemon, as one of the accounts this daemon holds on it. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 62;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
