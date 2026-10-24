// One dependency tool, named by its template and whose it is.

import { z } from "zod";
import { Owner } from "./owner.js";

/**
 * One dependency tool of the caller's: its template, by id, and
 * whose it is — see `Owner`. The pair names the tool. One JSON
 * object: `{"template":…,"owner":…}`.
 */
export const Dependency = z.object({
  /** The dependency template, by id: the hash of its canonical bytes. */
  template: z.string(),
  /** Whose the tool is. */
  owner: Owner,
});
export type Dependency = z.infer<typeof Dependency>;
