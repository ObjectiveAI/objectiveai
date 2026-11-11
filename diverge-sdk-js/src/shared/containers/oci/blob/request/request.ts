// What a blob ask names.

import { z } from "zod";
import { json } from "../../../../../wire/encode.js";

/**
 * The digest, and deliberately nothing else: no offset and no length,
 * and no name, because a blob is identified by its digest.
 */
export const Request = z.object({
  /** The blob's digest, `<algorithm>:<hex>`. */
  digest: z.string(),
});
export type Request = z.infer<typeof Request>;

/** Its JSON, and nothing in front of it: the tag that says which ask belongs to whichever frame carries it. */
export function encode(request: Request): Uint8Array {
  return json(request);
}
