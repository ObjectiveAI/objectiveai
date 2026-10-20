// What narrows a list of dependency tools.

import { z } from "zod";
import { Creator } from "../../../../../creator/creator.js";
import { Owner } from "../../../../../reference/owner.js";
import { Scope } from "../../../../../../shared/containers/dependencies/scope.js";
import { Mode } from "./mode.js";

/**
 * The filter over dependency tools: every member optional, and every one given
 * a condition a tool passes or does not. In a list request it is
 * flattened into the request and narrows what the daemon sends; in a
 * grant it is read as a test. A filter with no member given passes
 * everything. A member that lists candidates matches any one of them;
 * an empty list is absent, and matches everything.
 */
export const Filter = z.object({
  /** Of any one of these templates, by id. */
  ids: z.array(z.string()).optional(),
  /** Owned by any one of these: an agent, named either way, an agent template, or nobody. Absent when empty, and then any owner. */
  owners: z.array(Owner).optional(),
  /** Made by any one of these, directly. Absent when empty, and then made by anybody. */
  creators: z.array(Creator).optional(),
  /** Of any one of these modes, transient or a service. Absent when empty, and then either. */
  modes: z.array(Mode).optional(),
  /** A service whose `scope` is any one of these; a transient tool matches none. Absent when empty. */
  scopes: z.array(Scope).optional(),
  /** Whether some agent's last registration declared it: `true`, or none did, `false`; absent, either. */
  depended: z.boolean().optional(),
  /** Depended on by every one of these agents, by name. Absent when empty. */
  agents: z.array(z.string()).optional(),
  /** Whether its container runs now: `true`, or not, `false`; absent, either. */
  in_use: z.boolean().optional(),
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
