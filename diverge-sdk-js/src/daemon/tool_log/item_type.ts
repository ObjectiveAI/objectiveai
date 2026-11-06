// The kinds of item a tool's log holds, by name.

import { z } from "zod";

/** One kind of `Item`, as its `type` names it: what a request's `type` picks by. */
export const ItemType = z.enum([
  "deployed",
  "active",
  "inactive",
  "undeployed",
  "call",
  "call_result",
  "call_error",
  "call_interrupted",
  "read",
  "read_result",
  "read_error",
  "read_interrupted",
  "notification",
  "error",
  "command",
]);
export type ItemType = z.infer<typeof ItemType>;
