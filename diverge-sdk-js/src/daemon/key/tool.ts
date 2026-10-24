// One tool, once and for all.

import { z } from "zod";
import { Origin } from "./origin.js";
import { Owner } from "./owner.js";

/**
 * One tool of the caller's, once and for all: a record, by its fixed
 * `Origin` and its index, with its name beside when it has one; or
 * a dependency tool, by the template it runs from and whose it is.
 * Externally tagged: `{"record":{…}}` or `{"dependency":{…}}`.
 */
export const Tool = z.union([
  z.strictObject({
    record: z.object({
      /** What it was made with. */
      origin: Origin,
      /** Its number among all tools of the caller's ever made with that origin. */
      index: z.number().int().nonnegative(),
      /** Its name, if it has one. */
      name: z.string().optional(),
    }),
  }),
  z.strictObject({
    dependency: z.object({
      /** The dependency template it runs from, by id. */
      template: z.string(),
      /** Whose it is. */
      owner: Owner,
    }),
  }),
]);
export type Tool = z.infer<typeof Tool>;
