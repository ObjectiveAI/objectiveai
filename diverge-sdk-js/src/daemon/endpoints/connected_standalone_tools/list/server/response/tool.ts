// One connected tool, as the daemon holds it.

import { z } from "zod";
import { Creator } from "../../../../../../daemon/creator/creator.js";
import { Agent } from "../../../../../../daemon/key/agent.js";
import { Tool } from "../../../../../../daemon/reference/tool.js";
import { DaemonId } from "../../../../daemons/id.js";
import type { Keyed } from "../../../../../../shared/listing/keyed.js";

/**
 * One connected tool of the caller's: what the daemon knows of it.
 *
 * Everything here the daemon holds for the tool's life, so a list
 * costs no more than the tools it names. What the tool is made from —
 * its image, its limits, its mounts, its mode, the grants its
 * container declares — is the other daemon's, and unknown here.
 */
export const ConnectedStandaloneTool = z.object({
  /** The name, as its create gave it, if it gave one; absent for a tool made with none, which is reached by its daemon, its tool there and its index alone. */
  name: z.string().optional(),
  /** The daemon the tool is on, by the ID of this daemon's record of it, as the create named it. */
  daemon: DaemonId,
  /** The remote-daemon records the tool's connect is relayed through beyond `daemon`, as the create named them. Absent when empty: the tool is on `daemon`. */
  route: z.array(DaemonId).optional(),
  /** The tool, as that daemon names it and as the create named it. */
  tool: Tool,
  /** Its number among all connected tools of the caller's ever held from that daemon's tool, deleted ones included: the first is `1`, and no number is given twice. */
  index: z.number().int().nonnegative(),
  /** Who made it: the client, over an endpoint, or the agent or the tool of the client's that did so through the daemon. One `Creator`, the direct maker. */
  creator: Creator,
  /** When the create made it. */
  created: z.iso.datetime({ offset: true }),
  /** Whether the tool is active now: a call or a read the daemon routed into it in flight. Whether its connect is held is `deployed`. */
  active: z.boolean(),
  /** When that last changed; absent for a tool never asked. */
  last_active: z.iso.datetime({ offset: true }).optional(),
  /** Whether it is deployed now: the daemon holds the other daemon's connect on it. */
  deployed: z.boolean(),
  /** When that last changed; absent for one never deployed. */
  last_deployed: z.iso.datetime({ offset: true }).optional(),
  /** The agents it is attached to directly, by `attach_agent_instance`, each by its `key`, in the order they were attached. Absent when empty. */
  attached_agent_instances: z.array(Agent).optional(),
  /** The agent templates it is attached to, by `attach_agent_template`, each by its id, in the order they were attached. Absent when empty. */
  attached_agent_templates: z.array(z.string()).optional(),
  /** Its tags, sorted bytewise: what `tag` put on it and `untag` has not taken off. Absent when empty. */
  tags: z.array(z.string()).optional(),
});
export type ConnectedStandaloneTool = z.infer<typeof ConnectedStandaloneTool>;

/**
 * Keyed by the daemon, the tool there and the index, which together
 * name the connected tool once and for all. The name is not in it,
 * nor the route, which the three fix.
 */
export const key: Keyed<ConnectedStandaloneTool> = (tool) => ({ daemon: tool.daemon, tool: tool.tool, index: tool.index });
