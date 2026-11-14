// What narrows a list of images.

import { z } from "zod";
import { Identity } from "../../../../../agent_log/identity.js";

/**
 * The filter over the images in the daemon's Stores: every member
 * optional, and every one given a condition an image passes or does
 * not. In a list request it is flattened into the request and narrows
 * what the daemon sends. It is not what an images grant reaches by: a
 * grant reaches providers. A filter with no member given passes
 * everything. A member that lists candidates matches any one of them;
 * an empty list is absent, and matches everything.
 */
export const Filter = z.object({
  /** In the daemon's Store on any one of these providers. */
  providers: z.array(Identity).optional(),
  /** Any one of these digests, `<algorithm>:<hex>`, on whichever provider. */
  digests: z.array(z.string()).optional(),
  /** The earliest `created` to list, inclusive; absent, no earliest. */
  created_from: z.iso.datetime({ offset: true }).optional(),
  /** The latest `created` to list, inclusive; absent, no latest. */
  created_to: z.iso.datetime({ offset: true }).optional(),
});
export type Filter = z.infer<typeof Filter>;
