// What a `notifications` answers.

import { z } from "zod";
import { Notification } from "./notification.js";
import { DecodeError, jsonOf, splitTag } from "../../../../wire/decode.js";
import { ErrorData } from "../../error_data.js";

/**
 * What comes on the notifications channel: one notification per
 * frame, for the channel's life, or the server's error as the last
 * thing on it where it stops early. Nothing in the payload ends the
 * stream: the channel's own finish says there will be no more. A
 * payload leads with one byte saying which — `0` for
 * `notification`, `1` for `error` — and the rest is that variant's
 * JSON.
 */
export type Frame = { readonly notification: z.infer<typeof Notification> } | { readonly error: ErrorData };
// Annotated rather than inferred: MCP's result types name a `JSONValue`
// the package does not export, which a declaration could not refer to.
export const Frame: z.ZodType<Frame> = z.union([
  /** The server's answer. Tag `0`. */
  z.strictObject({ notification: Notification }),
  /** The server's error, whole. Tag `1`. */
  z.strictObject({ error: ErrorData }),
]);

const NOTIFICATION = 0;
const ERROR = 1;

/**
 * A frame out of its payload: the tag byte, then the variant's own
 * JSON, parsed by MCP's schema. A tag that names no variant is the
 * `unexpected_tag` error.
 */
export function decode(bytes: Uint8Array): Frame {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case NOTIFICATION:
      return { notification: Notification.parse(jsonOf(rest)) };
    case ERROR:
      return { error: ErrorData.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
