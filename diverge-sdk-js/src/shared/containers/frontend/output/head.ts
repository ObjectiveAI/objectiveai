// An HTTP response's head.

import { z } from "zod";

/** An HTTP response's head: its status, and its headers in their order, each a name and a value. */
export const Head = z.object({
  /** The status code. */
  status: z.number().int().min(0).max(65535),
  /** The headers, each `[name, value]`, in their order. */
  headers: z.array(z.tuple([z.string(), z.string()])).optional(),
});
export type Head = z.infer<typeof Head>;
