// Every agent, the dependency's own, those never active, or only these.

import { z } from "zod";
import { Tags } from "../tags.js";

/**
 * How far a grant over agents reaches: every agent there is, the
 * dependency's own agents, the agents that have never been active, or
 * only those the `Tags` let through. Flat on the wire: the string
 * `"any"`, the string `"self"`, the string `"never_active"`, or the
 * tags object as it is. `"self"` names nothing and reaches, for a
 * transient tool, the one agent it was deployed for, and, for a
 * service, every agent attached to it. `"never_active"` reaches every
 * agent no container has ever been started for, judged at each
 * request, so an agent leaves it the moment it first runs.
 */
export const Within = z.union([z.literal("any"), z.literal("self"), z.literal("never_active"), Tags]);
export type Within = z.infer<typeof Within>;
