// One dependency tool, as the daemon lists it.

import { z } from "zod";
import { Creator } from "../../../../../../daemon/creator/creator.js";
import { Owner } from "../../../../../../daemon/key/owner.js";
import { Template } from "../../../../../../shared/containers/dependencies/template.js";
import type { Key, Keyed } from "../../../../../../shared/listing/keyed.js";
import { Attached } from "./attached.js";
import { Run } from "./run.js";

/**
 * One dependency tool of the caller's: its template and its owner,
 * which together name it, when it was made, who made it, its tags, the
 * template whole, the agents that depend on it, and its run, if its
 * container is up. The tags are the tool's and not the template's: not
 * in it, and not in its hash.
 */
export const Listed = z.object({
  /** The template, by id: the lowercase hexadecimal SHA-256 of its canonical bytes, as `dependency_tools` states it. An agent calls the tool's MCP tools by the id's first eight characters, as a prefix. */
  id: z.string(),
  /** Whose the tool is, as the template's scope decides: the one agent, the one agent template, or nobody. See `key::Owner`. */
  owner: Owner,
  /** Its number among all dependency tools ever made from the template, whatever their owners: the first is `1`, and no number is given twice. The template and the index name the tool once and for all, as an events party does. */
  index: z.number().int().nonnegative(),
  /** When the first registration that declared the template under this owner made the tool. On the wire an RFC 3339 timestamp in UTC. */
  created: z.iso.datetime({ offset: true }),
  /** Who made it: the agent whose registration first declared the template under this owner. One `Creator`, the direct maker; the maker's own maker is on the maker's list item. */
  creator: Creator,
  /** Its tags, sorted bytewise: what `tag` put on it and `untag` has not taken off. Absent when empty. */
  tags: z.array(z.string()).optional(),
  /** The template, as the program declared it: the bytes the id is the hash of. See `Template`. */
  template: Template,
  /** The agents that depend on it now — every agent whose LAST registration declared the template under this owner, in the order they first did, each with when it last did: see `Attached`. At most one for an instance tool. Absent when empty. */
  agents: z.array(Attached).optional(),
  /** Its run, while its container is up: see `Run`. What a `delete` answers `InUse` for. Absent while it does not run. */
  run: Run.optional(),
});
export type Listed = z.infer<typeof Listed>;

/**
 * Keyed by the template's id and the owner, which together name the
 * dependency tool — the owner less an agent's name, which is carried
 * beside it and is not what names the agent.
 */
export const key: Keyed<Listed> = (listed) => ({ id: listed.id, owner: ownerKey(listed.owner) });

/** The owner as a key: the agent by template and index, the template by id, or `global`. */
function ownerKey(owner: Owner): Key {
  if (owner === "global") {
    return owner;
  }
  if ("agent" in owner) {
    return { agent: { template: owner.agent.template, index: owner.agent.index } };
  }
  return { agent_template: { template: owner.agent_template.template } };
}
