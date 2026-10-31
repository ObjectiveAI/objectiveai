// What narrows a list of agents.

import { z } from "zod";
import { Creator } from "../../../../../creator/creator.js";
import { Active } from "./active.js";

/**
 * The filter over agents: every member optional, and every one given
 * a condition an agent passes or does not. In a list request it is
 * flattened into the request and narrows what the daemon sends; in a
 * grant it is read as a test. A filter with no member given passes
 * everything. A member that lists candidates matches any one of them;
 * an empty list is absent, and matches everything.
 */
export const Filter = z.object({
  /** Any one of these names, as a create gave them. */
  names: z.array(z.string()).optional(),
  /** Made from any one of these templates, by id. */
  templates: z.array(z.string()).optional(),
  /** Made by any one of these, directly. Absent when empty, and then made by anybody. */
  creators: z.array(Creator).optional(),
  /** Whether active now — `true`, or not, `false` — or ever: `"ever"`, active at some time, or `"never"`, the agents that have never run. See `Active`. Absent, any. */
  active: Active.optional(),
  /** Whether deployed — its container up — or not; absent, either. */
  deployed: z.boolean().optional(),
  /** Depending on any one of these dependency tool templates, by id: an agent whose LAST registration declared one of them, as its `dependencies` say. Absent when empty, and then whatever it depends on, or nothing. */
  dependency_tool_templates: z.array(z.string()).optional(),
  /** Every one of these among the thing's tags. Absent when empty, and then any tags. */
  all_tags: z.array(z.string()).optional(),
  /** Any one of these among the thing's tags; with `all_tags`, both hold. Absent when empty. */
  any_tags: z.array(z.string()).optional(),
  /** The earliest `created` to list, inclusive; absent, no earliest. */
  created_from: z.iso.datetime({ offset: true }).optional(),
  /** The latest `created` to list, inclusive; absent, no latest. */
  created_to: z.iso.datetime({ offset: true }).optional(),
});
export type Filter = z.infer<typeof Filter>;
