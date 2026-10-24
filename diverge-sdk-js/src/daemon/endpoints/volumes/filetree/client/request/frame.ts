// Seeing what a volume holds: one snapshot of its tree.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Volume } from "../../../../../../daemon/reference/volume.js";

/**
 * Ask the daemon for one snapshot of a volume's tree, at a subtree of
 * it or whole. The daemon takes the volume at rest and asks the
 * provider for the tree, as the provider protocol's
 * `volumes.filetree` answers it, and answers with it once. Nothing is
 * watched and no change is sent.
 */
export const Frame = z.object({
  /** The volume: its provider and its name. */
  volume: Volume,
  /** The directory whose tree to send, as components from the volume's root: empty, the whole volume. A path at which a file is has no tree. Each component a name: not empty, not `.` or `..`, holding no `/` and no NUL. */
  path: z.array(z.string()),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 78;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
