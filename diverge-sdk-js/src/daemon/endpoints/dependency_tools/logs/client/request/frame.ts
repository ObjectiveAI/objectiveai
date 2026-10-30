// Reading a dependency tool's log.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Dependency } from "../../../../../../daemon/reference/dependency.js";
import { ItemType } from "../../../../../tool_log/item_type.js";

/**
 * Ask the daemon for a dependency tool's log: every item kept, narrowed by
 * where in the log, when, and what kind, the first `count` of them,
 * and then, with `watch`, every item as it is kept — read exactly as
 * an agent's log is (`agents.logs`), and ending as well when the tool
 * is deleted.
 */
export const Frame = z.object({
  /** The dependency tool: by its dependency template and its owner. */
  dependency: Dependency,
  /** The first `logs_index` to read, inclusive; absent, the log's first. */
  logs_index_from: z.number().int().nonnegative().optional(),
  /** The last `logs_index` to read, inclusive; absent, the log's last — and, watching, no last at all. */
  logs_index_to: z.number().int().nonnegative().optional(),
  /** The earliest `created` to read, inclusive; absent, the log's first. */
  created_from: z.iso.datetime({ offset: true }).optional(),
  /** The latest `created` to read, inclusive; absent, the log's last — and, watching, no last at all. */
  created_to: z.iso.datetime({ offset: true }).optional(),
  /** One kind of item, by its `type`; absent, every kind. */
  type: ItemType.optional(),
  /** How many items to send at most; once that many have been sent the scope finishes. `0` sends nothing. Absent, no cap. */
  count: z.number().int().nonnegative().optional(),
  /** Whether to watch: `true`, what matches now and then each item as it lands; `false` or absent, what matches now. */
  watch: z.boolean().optional(),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 113;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
