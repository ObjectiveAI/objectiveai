// Copying a file or a directory out of a volume to a destination.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Volume } from "../../../../../../daemon/reference/volume.js";
import { Destination } from "../../../../../../daemon/transfer/destination.js";

/**
 * Ask the daemon to copy a file or a directory out of a volume to a
 * destination: see `daemon.transfer.Destination` for what lands
 * where. The destination is another volume, or a container; the
 * same volume at another path is refused, the volume being taken
 * for the read while the write asks for it. The bytes never reach
 * the client: the daemon copies
 * on its own connections, holding the volume for the length of it;
 * an agent's or a created tool's container the destination names
 * that is not running is started for the operation and stopped when
 * it finishes, and a connected tool is joined for it and left.
 */
export const Frame = z.object({
  /** The volume: its provider and its name. */
  volume: Volume,
  /** The file or directory, as components from the volume's root: empty, the whole volume. Each component a name: not empty, not `.` or `..`, holding no `/` and no NUL. */
  path: z.array(z.string()),
  /** Where it lands. */
  to: Destination,
  /** Served by another daemon, as one of the accounts this daemon holds on it. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 75;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
