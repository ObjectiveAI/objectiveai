// One tool of the caller's, named any of three ways.

import { z } from "zod";

/**
 * One tool of the caller's: by its name, by its template and its
 * index, or — a connected tool — by the daemon and the tool it
 * joined, the tool as that daemon names it, recursively.
 * Untagged JSON, one object any way; an object with members of more
 * than one variant does not decode.
 */
export type Tool =
  | { name: string }
  | { template: string; index: number }
  | { daemon: string; tool: Tool };

export const Tool: z.ZodType<Tool> = z.lazy(() =>
  z.union([
    z.strictObject({ name: z.string() }),
    z.strictObject({ template: z.string(), index: z.number().int().nonnegative() }),
    z.strictObject({ daemon: z.string(), tool: Tool }),
  ]),
);
