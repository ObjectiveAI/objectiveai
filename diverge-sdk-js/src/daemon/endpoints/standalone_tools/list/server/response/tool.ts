// One tool, as the daemon lists it.

import { z } from "zod";
import { Creator } from "../../../../../../daemon/creator/creator.js";
import { Agent } from "../../../../../../daemon/key/agent.js";
import { Grant } from "../../../../../../shared/permission/grant.js";
import type { Keyed } from "../../../../../../shared/listing/keyed.js";
import { Origin } from "./origin.js";

/**
 * One tool of the caller's: what the daemon knows of it.
 *
 * Everything here the daemon holds for the tool's life, so a list costs
 * no more than the tools it names. What a created tool was made from is
 * its template's, named by id, and its mounts are the create's, and
 * neither is repeated here; what a connected tool is made from is the
 * other daemon's, and unknown here.
 */
export const Tool = z.object({
  /** The name, as its create or its connect gave it, if it gave one; absent for a tool made with none, which is reached by its template and its index, or by the daemon and tool it joined, alone. */
  name: z.string().optional(),
  /** Where it comes from, and what the daemon knows of its container — with the index that names it once and for all: see `Origin`. */
  origin: Origin,
  /** Who made it: the client, over an endpoint, or the agent or the tool of the client's that did so through the daemon. One `Creator`, the direct maker; the maker's own maker is on the maker's list item. */
  creator: Creator,
  /** When the create or the connect made it. */
  created: z.iso.datetime({ offset: true }),
  /** Whether the tool is active now: working, a call or a read in flight. Whether its container is up is `deployed`. */
  active: z.boolean(),
  /** When that last changed: the time the container started, if it runs; the time it last stopped, if it does not; absent for a tool that has never run. */
  /** When that last changed; absent for a tool never asked. */
  last_active: z.iso.datetime({ offset: true }).optional(),
  /** Whether its container is up now (a connected tool: the daemon holds a connect to it). */
  deployed: z.boolean(),
  /** When that last changed; absent for one never deployed. */
  last_deployed: z.iso.datetime({ offset: true }).optional(),
  /** The agents it is attached to directly, by `attach_agent_instance`, each by its `key` — its template and its index, with its name beside when it has one — in the order they were attached; empty for none. The agents list reports the same attachments from the other side, beside those an agent has through its template. */
  attached_agent_instances: z.array(Agent).optional(),
  /** The agent templates it is attached to, by `attach_agent_template`, each by its id, in the order they were attached; empty for none. Every agent of each has the tool. */
  attached_agent_templates: z.array(z.string()).optional(),
  /** What its container declared it may ask of the daemon, at its registration when it last started. Declared anew at every start. Absent when empty — none declared, never started, or connected. */
  permissions: z.array(Grant).optional(),
  /** Its tags, sorted bytewise: what `tag` put on it and `untag` has not taken off. Absent when empty. */
  tags: z.array(z.string()).optional(),
});
export type Tool = z.infer<typeof Tool>;

/**
 * Keyed by the origin less the members that may change while listed
 * — a created tool's mode, provider and container id — so a created
 * tool is its template and its index, a connected one the daemon,
 * the reference and its index. The name is not in it.
 */
export const key: Keyed<Tool> = (tool) =>
  tool.origin.kind === "created"
    ? { kind: "created", template: tool.origin.template, index: tool.origin.index }
    : { kind: "connected", daemon: tool.origin.daemon, tool: tool.origin.tool, index: tool.origin.index };
