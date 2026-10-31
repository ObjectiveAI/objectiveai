// One entry of a command log: its place, its time, and the item.

import { z } from "zod";
import { Item } from "./item.js";

/** One entry: its place in the log, when it was kept, and the command, flattened beside them. */
export const ItemWrapper = z.intersection(
  z.object({
    logs_index: z.number().int().nonnegative(),
    created: z.iso.datetime({ offset: true }),
  }),
  Item,
);
export type ItemWrapper = z.infer<typeof ItemWrapper>;
