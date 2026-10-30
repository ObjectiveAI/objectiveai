// Reading a log: an agent's or a tool's, the subject named.

import { z } from "zod";
import { json, tagged } from "../../../../../wire/encode.js";
import { Relay } from "../../../../../shared/relay.js";
import { Subject } from "./subject.js";
import { ItemType } from "./item_type.js";

/**
 * Ask the daemon for a log — an agent's or a tool's, the subject named: every item kept, narrowed by
 * where in the log, when, and what kind, the first `count` of them,
 * and then, with `watch`, every item as it is kept — historical then
 * live, with no marker between, an item at the seam sent exactly
 * once. A watch ends on its own when `count` is met, when the log
 * holds an item at or past `logs_index_to`, or when one is kept
 * later than `created_to` or the daemon's clock passes it; with none
 * of those, never. Spans are inclusive at both ends.
 */
export const Frame = z.object({
  /** Whose log. */
  of: Subject,
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
  /** How many items to send at most, counting what comes back and not what the filter reads; once that many have been sent the scope finishes, whether or not more would have matched. `0` sends nothing and finishes at once. Absent, no cap. */
  count: z.number().int().nonnegative().optional(),
  /** Whether to watch: `true`, the daemon sends what matches now and then runs the filter over each item as it lands, until the count is met, the filter can never match again, a cancel, or the subject's deletion; `false` or absent, the daemon sends what matches now and finishes. */
  watch: z.boolean().optional(),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 114;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
