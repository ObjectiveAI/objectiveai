// An image's DIVERGE.md, parsed.

import { z } from "zod";

/**
 * An image's `/DIVERGE.md`, parsed: the JSON frontmatter — the image's
 * name, the JSON Schema of the arguments the image's program accepts,
 * and whether the program serves a frontend — and the markdown body
 * beneath it.
 */
export const Document = z.object({
  /** The image's own name, as its author states it. */
  name: z.string(),
  /** The arguments' JSON Schema, a JSON object as the parse requires. */
  schema: z.json(),
  /** Whether the program serves a frontend beneath `/frontend`. Absent, it serves none. */
  frontend: z.boolean().optional(),
  /** The markdown beneath the frontmatter. */
  body: z.string(),
});
export type Document = z.infer<typeof Document>;
