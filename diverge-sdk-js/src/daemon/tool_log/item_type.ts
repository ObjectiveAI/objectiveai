// The kinds of item a tool's log holds, by name.

import { z } from "zod";

/** One kind of `Item`, as its `type` names it: what a request's `type` picks by. */
export const ItemType = z.enum(["active", "inactive", "call", "call_result", "call_error", "read", "read_result", "read_error", "notification", "error"]);
export type ItemType = z.infer<typeof ItemType>;
