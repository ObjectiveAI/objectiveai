// One tool, as the daemon lists it.

import { z } from "zod";
import { Creator } from "../../../../../../daemon/creator/creator.js";
import { Agent } from "../../../../../../daemon/key/agent.js";
import { Grant } from "../../../../../../shared/permission/grant.js";
import type { Keyed } from "../../../../../../shared/listing/keyed.js";
import { Provider } from "../../../../../agent_log/provider.js";
import { Mode } from "../../../templates/mode.js";

/**
 * One tool of the caller's: what the daemon knows of it.
 *
 * Everything here the daemon holds for the tool's life, so a list costs
 * no more than the tools it names. What a tool was made from is its
 * template's, named by id, and its mounts are the create's, and
 * neither is repeated here. A connected tool — another daemon's,
 * held here — is listed by `connected_standalone_tools.list`, and
 * never here.
 */
export const Tool = z.object({
  /** The name, as its create gave it, if it gave one; absent for a tool made with none, which is reached by its template and its index alone. */
  name: z.string().optional(),
  /** The template it was made from, by id. */
  template: z.string(),
  /** Its number among all tools of the caller's ever made from that template, deleted ones included; never given twice. */
  index: z.number().int().nonnegative(),
  /** How it is kept running, as the template says. */
  mode: Mode,
  /** The provider its container runs on, once it has run; absent before. */
  provider: Provider.optional(),
  /** Its container's id on that provider, while it runs; absent otherwise. */
  id: z.string().optional(),
  /** Who made it: the client, over an endpoint, or the agent or the tool of the client's that did so through the daemon. One `Creator`, the direct maker; the maker's own maker is on the maker's list item. */
  creator: Creator,
  /** When the create or the connect made it. */
  created: z.iso.datetime({ offset: true }),
  /** Whether the tool is active now: working, a call or a read in flight. Whether its container is up is `deployed`. */
  active: z.boolean(),
  /** When that last changed: the time the container started, if it runs; the time it last stopped, if it does not; absent for a tool that has never run. */
  /** When that last changed; absent for a tool never asked. */
  last_active: z.iso.datetime({ offset: true }).optional(),
  /** Whether its container is up now. */
  deployed: z.boolean(),
  /** When that last changed; absent for one never deployed. */
  last_deployed: z.iso.datetime({ offset: true }).optional(),
  /** The agents it is attached to directly, by `attach_agent_instance`, each by its `key` — its template and its index, with its name beside when it has one — in the order they were attached; empty for none. The agents list reports the same attachments from the other side, beside those an agent has through its template. */
  attached_agent_instances: z.array(Agent).optional(),
  /** The agent templates it is attached to, by `attach_agent_template`, each by its id, in the order they were attached; empty for none. Every agent of each has the tool. */
  attached_agent_templates: z.array(z.string()).optional(),
  /** What its container declared it may ask of the daemon, at its registration when it last started. Declared anew at every start. Absent when empty — none declared, or never started. */
  permissions: z.array(Grant).optional(),
  /** Its tags, sorted bytewise: what `tag` put on it and `untag` has not taken off. Absent when empty. */
  tags: z.array(z.string()).optional(),
});
export type Tool = z.infer<typeof Tool>;

/**
 * Keyed by its template and its index, which together name it once
 * and for all, as the pair `[template, index]`. The members that may
 * change while listed — the mode, the provider, the container id —
 * are not in it, nor the name.
 */
export const key: Keyed<Tool> = (tool) => [tool.template, tool.index];
