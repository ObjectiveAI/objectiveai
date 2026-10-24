// The image a container runs.

import { z } from "zod";
import { Reference } from "./reference.js";

/**
 * The image, by its manifest digest and the references under which
 * its bytes may be fetched. A digest cannot be repointed at different
 * content, so an image means the same bytes every time, wherever
 * they come from: that is the whole of what the wire asserts. The
 * references are not the image — the same bytes sit under different
 * names on different registries — and an id that hashes an image
 * hashes it with none; see `hashed`.
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
