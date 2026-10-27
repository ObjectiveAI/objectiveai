// An image's DIVERGE.md, parsed.

import { z } from "zod";

/**
 * An image's `/DIVERGE.md`, parsed: the JSON frontmatter, which is the
 * JSON Schema of the arguments the image's program accepts, and the
 * markdown body beneath it.
 */
export const Document = z.object({
  /** The arguments' JSON Schema, a JSON object as the parse requires. */
  schema: z.json(),
  /** The markdown beneath the frontmatter. */
  body: z.string(),
});
export type Document = z.infer<typeof Document>;
