// Send the content of one file: what the daemon asks the client for.

import { z } from "zod";
import { jsonOf } from "../../../../../../wire/decode.js";
import { json } from "../../../../../../wire/encode.js";

/**
 * Send the content of one file: what the daemon asks the client for,
 * and the only thing. The bytes travel as responses on a channel the
 * daemon opened — one per file of a directory, one for a file — and
 * the client answers with the file's pieces and finishes. An object
 * and no tag: one thing to ask for is an object, and the scope says
 * which upload.
 */
export const Frame = z.object({
  /** Which file: its path as the request named it, for a directory; absent for a file, which has the one. */
  path: z.string().optional(),
});
export type Frame = z.infer<typeof Frame>;

/** JSON, and nothing in front of it: the payload is the object. */
export function encode(frame: Frame): Uint8Array {
  return json(frame);
}

/** The object, parsed; a payload that is not one throws. */
export function decode(bytes: Uint8Array): Frame {
  return Frame.parse(jsonOf(bytes));
}
