// One agent template, as the daemon lists it.

import { z } from "zod";
import { Creator } from "../../../../../../../daemon/creator/creator.js";
import { Tool } from "../../../../../../../daemon/key/tool.js";
import type { Keyed } from "../../../../../../../shared/listing/keyed.js";
import { Template } from "../../../template.js";

/**
 * One template of the caller's: its id, when it was made, who made it,
 * its tags, the tools attached to it, and the template whole. The tags
 * and the attachments are the caller's and not the template's: not in
 * it, and not in its hash.
 */
export const Listed = z.object({
  /** The id: the template's hash, as `templates` states it. */
  id: z.string(),
  /** When the first create made it. On the wire an RFC 3339 timestamp in UTC. A template made again — answered `Exists`, or made anew after a delete — keeps it, as it keeps its creator. */
  created: z.iso.datetime({ offset: true }),
  /** Who made it: the client, over an endpoint, or the agent or the tool of the client's that did so through the daemon. One `Creator`, the direct maker; the maker's own maker is on the maker's list item. A template made again — answered `Exists`, or made anew after a delete — is the first maker's still. */
  creator: Creator,
  /** Its tags, sorted bytewise: what `tag` put on it and `untag` has not taken off. Absent when empty. */
  tags: z.array(z.string()).optional(),
  /** The `standalone_tools` attached to it by `attach_agent_template`, which every agent made from it has, each by its `key`, in the order they were attached; empty for none. */
  tools: z.array(Tool).optional(),
  /** The template, as it was handed in: the bytes the id is the hash of. */
  template: Template,
});
export type Listed = z.infer<typeof Listed>;

/** Keyed by id. */
export const key: Keyed<Listed> = (listed) => listed.id;
