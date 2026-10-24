// The image a template names.

import { z } from "zod";
import { Reference } from "./reference.js";

/**
 * The image, by its manifest digest and the references under which
 * its bytes may be fetched. The digest is the image: it cannot be
 * repointed at different content. The references are not; an id
 * that hashes an image hashes it with none, see `hashed`.
 */
export const Image = z.object({
  /** The manifest digest, `<algorithm>:<hex>`. */
  digest: z.string(),
  /** Where the bytes may be fetched, each a registry and a path there. Absent when there are none. */
  references: z.array(Reference).optional(),
});
export type Image = z.infer<typeof Image>;

/** The image as an id hashes it: the digest with no references. */
export function hashed(image: Image): Image {
  return { digest: image.digest };
}
