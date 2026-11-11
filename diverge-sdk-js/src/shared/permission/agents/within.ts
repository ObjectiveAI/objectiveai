// Every agent, the tool's attached agents, those never active, or only these.

import { z } from "zod";
import { Tags } from "../tags.js";

/**
 * How far a grant over agents reaches: every agent there is, the
 * agents attached to the tool holding the grant, the agents that have
 * never been active, or only those the `Tags` let through. Flat on the
 * wire: the string `"any"`, the string `"attached"`, the string
 * `"never_active"`, or the tags object as it is. `"attached"` names
 * nothing and reaches, for a standalone tool, the agents it is
 * attached to directly and every agent of every template it is
 * attached to; for a dependency tool, the agents attached to it — for
 * a transient tool the one agent it was deployed for. `"never_active"` reaches every
 * agent no container has ever been started for, judged at each
 * request, so an agent leaves it the moment it first runs.
 */
export const Within = z.union([z.literal("any"), z.literal("attached"), z.literal("never_active"), Tags]);
export type Within = z.infer<typeof Within>;
