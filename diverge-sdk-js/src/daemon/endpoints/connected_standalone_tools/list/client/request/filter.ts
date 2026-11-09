// What narrows a list of connected tools.

import { z } from "zod";
import { Creator } from "../../../../../creator/creator.js";
import { DaemonId } from "../../../../daemons/id.js";

/**
 * The filter over connected tools: every member optional, and every
 * one given a condition a tool passes or does not. In a list request
 * it is flattened into the request and narrows what the daemon sends;
 * in a grant it is read as a test. A filter with no member given
 * passes everything. A member that lists candidates matches any one
 * of them; an empty list is absent, and matches everything.
 */
export const Filter = z.object({
  /** Any one of these names, as a create gave them. */
  names: z.array(z.string()).optional(),
  /** On any one of these daemons, by the ID of this daemon's record of each, as the create named it — the tool's `daemon`. Absent when empty, and then on any. */
  daemons: z.array(DaemonId).optional(),
  /** Made by any one of these, directly. Absent when empty, and then made by anybody. */
  creators: z.array(Creator).optional(),
  /** Whether active — a call or a read the daemon routed into it in flight — `true`, or not, `false`; absent, either. */
  active: z.boolean().optional(),
  /** Whether deployed — its connect held — or not; absent, either. */
  deployed: z.boolean().optional(),
  /** Attached directly to every one of these agents, by name. Absent when empty. */
  attached_agent_instances: z.array(z.string()).optional(),
  /** Attached to every one of these agent templates, by id. Absent when empty. */
  attached_agent_templates: z.array(z.string()).optional(),
  /** Every one of these among the tool's tags. Absent when empty, and then any tags. */
  all_tags: z.array(z.string()).optional(),
  /** Any one of these among the tool's tags; with `all_tags`, both hold. Absent when empty. */
  any_tags: z.array(z.string()).optional(),
  /** The earliest `created` to list, inclusive; absent, no earliest. */
  created_from: z.iso.datetime({ offset: true }).optional(),
  /** The latest `created` to list, inclusive; absent, no latest. */
  created_to: z.iso.datetime({ offset: true }).optional(),
});
export type Filter = z.infer<typeof Filter>;
