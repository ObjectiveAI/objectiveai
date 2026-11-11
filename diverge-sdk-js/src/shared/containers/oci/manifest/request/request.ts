// What a manifest ask names.

import { z } from "zod";
import { json } from "../../../../../wire/encode.js";

/**
 * The digest, and deliberately nothing else: a manifest under one
 * name is the same manifest under another, and the media type is the
 * answer's to say.
 */
export const Request = z.object({
  /** The manifest's digest, `<algorithm>:<hex>`. */
  digest: z.string(),
});
export type Request = z.infer<typeof Request>;

/** Its JSON, and nothing in front of it: the tag that says which ask belongs to whichever frame carries it. */
export function encode(request: Request): Uint8Array {
  return json(request);
}
