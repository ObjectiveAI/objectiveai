// Uploading files into an agent's container.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Agent } from "../../../../../../daemon/reference/agent.js";

/**
 * Ask the daemon to put files into an agent's container: one file at
 * a path, or a directory of files at a path. JSON-tagged by `kind`.
 * A `file` names the file's destination, and the daemon opens one
 * content channel for it. A `directory` names the directory's
 * destination and every file in it, by path from the directory's
 * root, `/`-separated, no component empty, `.` or `..`, no path
 * twice, and at least one; the daemon opens one content channel per
 * path, and each file lands at the destination joined with its
 * path, replaced whole, every parent made. A container that is not
 * running is started for the operation and stopped when it finishes.
 */
export const Frame = z.discriminatedUnion("kind", [
  /** One file. */
  z.object({
    kind: z.literal("file"),
    /** The agent: by name, or by template and index. */
    agent: Agent,
    /** The file's destination, as components from the container's root; at least one, each a name: not empty, not `.` or `..`, holding no `/` and no NUL. */
    path: z.array(z.string()),
    /** Served by another daemon, as one of the accounts this daemon holds on it. Absent, served here. */
    relay: Relay.optional(),
  }),
  /** One directory of files. */
  z.object({
    kind: z.literal("directory"),
    /** The agent: by name, or by template and index. */
    agent: Agent,
    /** The directory's destination, as components from the container's root; empty, the root itself. Each a name, as a file's. */
    path: z.array(z.string()),
    /** The paths of its files, from the directory's root. */
    files: z.array(z.string()),
    /** Served by another daemon, as one of the accounts this daemon holds on it. Absent, served here. */
    relay: Relay.optional(),
  }),
]);
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 60;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
