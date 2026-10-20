// The two tagging actions.

import { z } from "zod";

/**
 * Putting tags on, or taking them off: the actions of a tagging
 * grant, the same two for every kind that carries tags. Snake case
 * on the wire: `"tag"`, `"untag"`.
 */
export const Tagging = z.enum(["tag", "untag"]);
export type Tagging = z.infer<typeof Tagging>;
