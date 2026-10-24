// Downloading a file or a directory out of a volume.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Volume } from "../../../../../../daemon/reference/volume.js";

/**
 * Ask the daemon to send a file or a directory out of a volume. What
 * comes back is addressed relative to the path named: a file's
 * chunks carry the empty path, a directory's each file's path within
 * it — see `daemon.download.Chunk`. The volume is taken at rest and
 * held for the length of the download, free after; nothing is
 * started for it.
 */
export const Frame = z.object({
  /** The volume: its provider and its name. */
  volume: Volume,
  /** The file or directory, as components from the volume's root: empty, the whole volume. Each component a name: not empty, not `.` or `..`, holding no `/` and no NUL. */
  path: z.array(z.string()),
  /** Served by another daemon, as one of the accounts this daemon holds on it. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 73;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
