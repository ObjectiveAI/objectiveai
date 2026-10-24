// One entry of a log: its place, its time, and the item.

import { z } from "zod";
import { Item } from "./item.js";

/**
 * One entry: its place in the log, when it was kept, and the `Item` it
 * holds — the item's own members flattened beside the two of the log's,
 * so an entry reads as the chunk or error it is with `logs_index` and
 * `created` added. Which message a user part is of, its own `key` says;
 * who sent that message, its `sender`.
 *
 * `logs_index` rather than `id`, because a tool call and a tool
 * response carry an `id` of their own, and flattening would put the two
 * in one object. `created` is nobody else's.
 */
export const ItemWrapper = z.intersection(
  z.object({
    /** The item's index: counts up by one per item of the agent's log, from `1`, and never repeats. What the request's `logs_index_from` and `logs_index_to` span. */
    logs_index: z.number().int().nonnegative(),
    /** When the daemon kept the item. On the wire an RFC 3339 timestamp in UTC, `2026-09-21T15:04:05.123456Z`. What the request's `created_from` and `created_to` span. */
    created: z.iso.datetime({ offset: true }),
  }),
  Item,
);
export type ItemWrapper = z.infer<typeof ItemWrapper>;
