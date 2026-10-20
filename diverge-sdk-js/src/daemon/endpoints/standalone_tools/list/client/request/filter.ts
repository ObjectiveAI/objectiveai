// What narrows a list of tools.

import { z } from "zod";
import { Creator } from "../../../../../creator/creator.js";
import { Mode } from "../../../templates/mode.js";
import { Kind } from "./kind.js";

/**
 * The filter over tools: every member optional, and every one given
 * a condition a tool passes or does not. In a list request it is
 * flattened into the request and narrows what the daemon sends; in a
 * grant it is read as a test. A filter with no member given passes
 * everything. A member that lists candidates matches any one of them;
 * an empty list is absent, and matches everything.
 */
export const Filter = z.object({
  /** Any one of these names, as a create or a connect gave them. */
  names: z.array(z.string()).optional(),
  /** A created tool made from any one of these templates, by id; a connected tool matches none. */
  templates: z.array(z.string()).optional(),
  /** Made by any one of these, directly. Absent when empty, and then made by anybody. */
  creators: z.array(Creator).optional(),
  /** Created by the daemon or connected to somebody else's; absent, either. */
  kind: Kind.optional(),
  /** Made from a template whose `mode` is any one of these; a connected tool passes none. Absent when empty, and then either. */
  modes: z.array(Mode).optional(),
  /** Whether active — its container running, or its connect scope held — `true`, or not, `false`; absent, either. */
  active: z.boolean().optional(),
  /** Attached directly to every one of these agents, by name. Absent when empty. */
  attached_agent_instances: z.array(z.string()).optional(),
  /** Attached to every one of these agent templates, by id. Absent when empty. */
  attached_agent_templates: z.array(z.string()).optional(),
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
