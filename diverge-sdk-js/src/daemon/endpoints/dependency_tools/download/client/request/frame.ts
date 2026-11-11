// Downloading a file or a directory out of a dependency tool's container.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Dependency } from "../../../../../../daemon/reference/dependency.js";

/**
 * Ask the daemon to send a file or a directory out of a running
 * dependency tool's container. What comes back is addressed
 * relative to the path named: a file's chunks carry the empty path,
 * a directory's each file's path within it — see
 * `daemon.download.Chunk`. A tool that does not run is `not_found`;
 * nothing is started.
 */
export const Frame = z.object({
  /** The tool: its template by id, and whose it is. */
  dependency: Dependency,
  /** The file or directory, as components from the container's root: empty, the whole root. Each component a name: not empty, not `.` or `..`, holding no `/` and no NUL. */
  path: z.array(z.string()),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 95;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
