// Copying a file or a directory out of a tool's container to a destination.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Tool } from "../../../../../../daemon/reference/tool.js";
import { Destination } from "../../../../../../daemon/transfer/destination.js";

/**
 * Ask the daemon to copy a file or a directory out of a tool's
 * container to a destination: see `daemon.transfer.Destination` for
 * what lands where. The bytes never reach the client: the daemon
 * copies on its own connections. A container at either end that is
 * not running is started for the operation and stopped when it
 * finishes; a connected tool is joined for it and left; a
 * dependency tool that is not running is `no_destination`.
 */
export const Frame = z.object({
  /** The tool: by name, by template and index, or another daemon's by the daemon, the account and its reference there. */
  tool: Tool,
  /** The file or directory, as components from the container's root: empty, the whole root. Each component a name: not empty, not `.` or `..`, holding no `/` and no NUL. */
  path: z.array(z.string()),
  /** Where it lands. */
  to: Destination,
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 64;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
