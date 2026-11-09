// One standalone tool of the caller's, named either way.

import { z } from "zod";

/**
 * One standalone tool of the caller's: by its name, or by its
 * template and its index. Untagged JSON, one object either way; an
 * object with members of both does not decode. A connected tool is
 * named by `ConnectedStandaloneTool`, never by this.
 */
export const Tool = z.union([
  /** By name: `{"name":…}`. A tool given none is not reached this way. */
  z.strictObject({ name: z.string() }),
  /** By template and index: `{"template":…,"index":…}`. */
  z.strictObject({ template: z.string(), index: z.number().int().nonnegative() }),
]);
export type Tool = z.infer<typeof Tool>;
