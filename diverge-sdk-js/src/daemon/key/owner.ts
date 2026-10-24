// Whose a dependency tool is: one agent's, one agent template's, or
// everyone's.

import { z } from "zod";
import { Agent } from "./agent.js";

/**
 * Whose a dependency tool is, as its template's `scope` named it:
 * the one agent, once and for all; every agent made from the
 * template; or every agent that declares it. Externally tagged:
 * `{"agent":{"template":…,"index":…}}`,
 * `{"agent_template":{"template":…}}`, or the string `"global"`.
 */
export const Owner = z.union([
  z.strictObject({ agent: Agent }),
  z.strictObject({ agent_template: z.object({ template: z.string() }) }),
  z.literal("global"),
]);
export type Owner = z.infer<typeof Owner>;
