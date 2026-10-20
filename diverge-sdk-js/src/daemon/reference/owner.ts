// Whose a dependency tool is: one agent's, one agent template's, or
// everyone's.

import { z } from "zod";
import { Agent } from "./agent.js";

/**
 * Whose a dependency tool is, as its template's `scope` names it:
 * the one agent it is for, named either way; the agent template
 * whose agents it is for; or nobody in particular. Externally
 * tagged: `{"agent":…}`, `{"agent_template":{"template":…}}`, or
 * the string `"global"`. The daemon answers the once-and-for-all
 * form, as `key.Owner`.
 */
export const Owner = z.union([
  z.strictObject({ agent: Agent }),
  z.strictObject({ agent_template: z.object({ template: z.string() }) }),
  z.literal("global"),
]);
export type Owner = z.infer<typeof Owner>;
