// One agent, as the daemon lists it.

import { z } from "zod";
import { Creator } from "../../../../../../daemon/creator/creator.js";
import { Tool } from "../../../../../../daemon/key/tool.js";
import type { Keyed } from "../../../../../../shared/listing/keyed.js";
import { Provider } from "../../../../../agent_log/provider.js";

/**
 * One agent of the caller's: what the daemon knows of it without
 * reading its log.
 *
 * Everything here the daemon holds for the agent's life, so a list
 * costs no more than the agents it names. What an agent has SAID is its
 * `logs`, read separately; what it was made from is its template's,
 * named by id, and its mounts are the create's, and neither is repeated
 * here.
 */
export const Agent = z.object({
  /** The name, as its create gave it, if it gave one; absent for an agent made with none, which is reached by its template and its index alone. */
  name: z.string().optional(),
  /** The template it was made from, by id: the hash a `templates.create` answered, and what a `templates.list` names it by. */
  template: z.string(),
  /** Its number among all agents of the caller's ever made from that template, deleted ones included: the first made is `1`, each after is one more, and no number is given twice. The template and the index together name the agent once and for all, where a name is free again once the agent is deleted. */
  index: z.number().int().nonnegative(),
  /** Who made it: the client, over an endpoint, or the agent or the tool of the client's that did so through the daemon. One `Creator`, the direct maker; the maker's own maker is on the maker's list item. */
  creator: Creator,
  /** When the create made it. */
  created: z.iso.datetime({ offset: true }),
  /** Whether the agent is active now: working, a loop running in it. Whether its container is up is `deployed`. */
  active: z.boolean(),
  /** When that last changed: the time its loop began, if one runs; the time its last loop ended, if not; absent for an agent that has never run a loop. */
  last_active: z.iso.datetime({ offset: true }).optional(),
  /** Whether its container is up now (a connected tool: the daemon holds a connect to it). */
  deployed: z.boolean(),
  /** When that last changed; absent for one never deployed. */
  last_deployed: z.iso.datetime({ offset: true }).optional(),
  /** The provider it is deployed on, or was last; absent for an agent never deployed. See `Provider`. */
  provider: Provider.optional(),
  /** The `logs_index` of its latest log item: how long its log is, and where a read that wants only what comes next starts. `0` for an empty log. */
  logs_index: z.number().int().nonnegative(),
  /** The `standalone_tools` it has: those attached to it directly, in the order they were attached, then those attached to its template, in theirs, one it has both ways once — each by its `key`, its origin and its index, with its name beside when it has one; empty for an agent with none. The tools list reports the same attachments from the other side. */
  tools: z.array(Tool).optional(),
  /** The dependency templates its LAST registration declared, by id, in the order it first declared them, each once: what it depends on now, as `dependency_tools` lists the same attachments from the other side. A registration replaces them whole. Absent when empty — an agent that never registered, or declared nothing. */
  dependencies: z.array(z.string()).optional(),
  /** Its tags, sorted bytewise: what `tag` put on it and `untag` has not taken off. Absent when empty. */
  tags: z.array(z.string()).optional(),
});
export type Agent = z.infer<typeof Agent>;

/** Keyed by template and index: the name is not in the key, since a name may be given again once its holder is deleted. */
export const key: Keyed<Agent> = (agent) => ({ template: agent.template, index: agent.index });
