// One entry of a tool's log: its place, its time, and the item.

import { z } from "zod";
import { Item } from "./item.js";

/** One entry: its place in the log, when it was kept, and the `Item` it holds, flattened beside them. */
export const ItemWrapper = z.intersection(
  z.object({
    /** The item's index: counts up by one per item of the tool's log, from `1`, and never repeats. What an answer names its ask by. */
    logs_index: z.number().int().nonnegative(),
    /** When the daemon kept the item, an RFC 3339 timestamp in UTC. */
    created: z.iso.datetime({ offset: true }),
  }),
  Item,
);
export type ItemWrapper = z.infer<typeof ItemWrapper>;
