// What narrows a list of volumes.

import { z } from "zod";
import { Mode } from "../../../../../../provider/endpoints/volumes/mode.js";
import { Identity } from "../../../../../agent_log/identity.js";

/**
 * The filter over volumes: every member optional, and every one given
 * a condition a volume passes or does not. In a list request it is
 * flattened into the request and narrows what the daemon sends; in a
 * grant it is read as a test. A filter with no member given passes
 * everything. A member that lists candidates matches any one of them;
 * an empty list is absent, and matches everything.
 */
export const Filter = z.object({
  /** Held by any one of these providers. */
  providers: z.array(Identity).optional(),
  /** Any one of these names, on whichever provider. */
  names: z.array(z.string()).optional(),
  /** In any one of these modes. */
  modes: z.array(Mode).optional(),
  /** Whether some agent or tool of the daemon's names it in its mounts: `true`, or none, `false`; absent, either. */
  mounted: z.boolean().optional(),
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
