// What opens a frontend request: its method, its path and its headers.

import { z } from "zod";

/**
 * One HTTP request's head: the method, the path with its query —
 * `/frontend` or beneath it — and the headers in their order, each a
 * name and a value; a name may repeat.
 */
export const Head = z.object({
  /** The method, as HTTP writes it: `GET`, `POST`, …. */
  method: z.string(),
  /** The path and the query, as the request line carries them: `/frontend/items?page=2`. */
  path: z.string(),
  /** The headers, each `[name, value]`, in their order. */
  headers: z.array(z.tuple([z.string(), z.string()])).optional(),
});
export type Head = z.infer<typeof Head>;
