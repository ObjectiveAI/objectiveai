// What a prove ask names.

import { z } from "zod";
import { json } from "../../../../../wire/encode.js";

/**
 * The platform and the nonce: everything the proof is computed over
 * that the answering side does not already hold. The image is not
 * among them: it is the one the scope carrying the ask names.
 */
export const Request = z.object({
  /**
   * When the digest of the scope's image names an index, the digest
   * of the one manifest in it whose image is meant, `<algorithm>:<hex>`;
   * absent when it names an image manifest.
   */
  platform: z.string().optional(),
  /** The nonce: 32 bytes, fresh, as 64 hexadecimal characters — lowercase as written, either case as read. */
  nonce: z.string().regex(/^[0-9a-fA-F]{64}$/),
});
export type Request = z.infer<typeof Request>;

/** Its JSON, and nothing in front of it: the tag that says which ask belongs to whichever frame carries it. */
export function encode(request: Request): Uint8Array {
  return json(request);
}
