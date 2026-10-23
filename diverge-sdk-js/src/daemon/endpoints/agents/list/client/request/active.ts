// Which agents the filter's `active` lets through.

import { z } from "zod";

/**
 * The two words of `Active`: whether an agent has ever been active.
 * `"ever"`: active at some time; `"never"`: never active.
 */
export const History = z.enum(["ever", "never"]);
export type History = z.infer<typeof History>;

/**
 * The `active` member of the agents filter: now, or ever. Untagged on
 * the wire, a boolean or one of two words. `true`: active now, a loop
 * running in it; `false`: not active now. `"ever"`: active at some
 * time — a container was started for it, which is what gives it a
 * `last_active`; `"never"`: never active, exactly the agents that
 * have never run.
 */
export const Active = z.union([z.boolean(), History]);
export type Active = z.infer<typeof Active>;
