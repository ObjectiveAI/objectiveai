// Uploading files into a dependency tool's container.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Dependency } from "../../../../../../daemon/reference/dependency.js";

/**
 * Ask the daemon to put files into a running dependency tool's
 * container: one file at a path, or a directory of files at a path.
 * JSON-tagged by `kind`. A `file` names the file's destination, and
 * the daemon opens one content channel for it. A `directory` names
 * the directory's destination and every file in it, by path from
 * the directory's root, `/`-separated, no component empty, `.` or
 * `..`, no path twice, and at least one; the daemon opens one
 * content channel per path, and each file lands at the destination
 * joined with its path, replaced whole, every parent made. A tool
 * that does not run is `not_found`; nothing is started.
 */
export const Frame = z.discriminatedUnion("kind", [
  /** One file. */
  z.object({
    kind: z.literal("file"),
    /** The tool: its template by id, and whose it is. */
    dependency: Dependency,
    /** The file's destination, as components from the container's root; at least one, each a name: not empty, not `.` or `..`, holding no `/` and no NUL. */
    path: z.array(z.string()),
    /** Served by another daemon: see `Relay`. Absent, served here. */
    relay: Relay.optional(),
  }),
  /** One directory of files. */
  z.object({
    kind: z.literal("directory"),
    /** The tool: its template by id, and whose it is. */
    dependency: Dependency,
    /** The directory's destination, as components from the container's root; empty, the root itself. Each a name, as a file's. */
    path: z.array(z.string()),
    /** The paths of its files, from the directory's root. */
    files: z.array(z.string()),
    /** Served by another daemon: see `Relay`. Absent, served here. */
    relay: Relay.optional(),
  }),
]);
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 96;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
