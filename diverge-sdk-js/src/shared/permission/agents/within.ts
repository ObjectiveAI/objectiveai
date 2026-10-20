// Every agent, the dependency's own, or only these.

import { z } from "zod";
import { Tags } from "../tags.js";

/**
 * How far a grant over agents reaches: every agent there is, the
 * dependency's own agents, or only those the `Tags` let through.
 * Flat on the wire: the string `"any"`, the string `"self"`, or the
 * tags object as it is. `"self"` is the one word the agents kind has
 * that no other has: it names nothing and reaches, for a transient
 * tool, the one agent it was deployed for, and, for a service, every
 * agent attached to it.
 */
export const Within = z.union([z.literal("any"), z.literal("self"), Tags]);
export type Within = z.infer<typeof Within>;
