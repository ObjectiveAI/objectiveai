// One image in a Store of the daemon's, as its provider lists it.

import { z } from "zod";
import { Identity } from "../../../../../agent_log/identity.js";
import { Description } from "../../../../../../shared/description.js";
import type { Keyed } from "../../../../../../shared/listing/keyed.js";

/**
 * One image the daemon owns: the provider whose Store it is in, and
 * what that provider's listing says of it — its digest, what was said
 * of it, its size, and when the daemon came to own it. No creator and
 * no tags: an image is no record of the daemon's, and the Store on a
 * provider is the daemon's own. The digest is what a template's image
 * names.
 */
export const Image = z.object({
  /** The provider whose Store of the daemon's holds it. */
  provider: Identity,
  /** The manifest digest, `<algorithm>:<hex>`: the whole of how it is named, with the provider. */
  digest: z.string(),
  /** What was said of the image by the build or the push that gained it. Absent, nothing was said. */
  description: Description.optional(),
  /** Its size in BYTES, as the provider counts it against the Store's capacity. */
  bytes: z.number().int().nonnegative(),
  /** When the daemon came to own it, as the provider says: an RFC 3339 timestamp in UTC. */
  created: z.iso.datetime({ offset: true }),
});
export type Image = z.infer<typeof Image>;

/** Keyed by the provider and the digest: one image is one digest, owned once on a provider. */
export const key: Keyed<Image> = (image) => [image.provider, image.digest];
