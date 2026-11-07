// A listen's answer: a stream kept open.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";
import { Identity } from "../../../../../../shared/identity.js";
import { base64 } from "../../../../../../shared/containers/exec/response/base64.js";

/**
 * A listen's answer, as JSON writes it: a message, its body in standard
 * base64; the count of messages a listener missed; forbidden; or a
 * failure. What the stream yields is the `Item`, the same frame with
 * the body's bytes. Zero or more `message`s and `skipped`s for the
 * scope's life, in the order the daemon took the messages; the finish
 * follows the client's cancel or exactly one `error`; or the scope is
 * exactly one `forbidden`.
 */
export const Frame = z.union([
  /** One message sent to the daemon: the provider it came through, who sent it, and its body. Tag `0`. */
  z.strictObject({ message: z.strictObject({ provider: Identity, from: Identity, body: z.base64() }) }),
  /** The listener fell further behind than the daemon keeps, and missed this many messages. Tag `1`. */
  z.strictObject({ skipped: z.strictObject({ count: z.number().int().nonnegative() }) }),
  /** The account the request is served for holds no grant allowing it; nothing was sent. Tag `2`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `3`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

/** One message as the stream yields it: its body as bytes. */
export interface Message {
  readonly provider: Identity;
  readonly from: Identity;
  readonly body: Uint8Array;
}

/** One answer as the stream yields it: the frame with the message's bytes. */
export type Item =
  | { readonly message: Message }
  | { readonly skipped: { readonly count: number } }
  | "forbidden"
  | { readonly error: Error };

const MESSAGE = 0;
const SKIPPED = 1;
const FORBIDDEN = 2;
const ERROR = 3;

/** The two identities a message's payload writes ahead of its body. */
const Head = z.object({ provider: Identity, from: Identity });

/** The count a `skipped` carries. */
const Skipped = z.object({ count: z.number().int().nonnegative() });

/**
 * An item out of its payload: the tag byte, then, for a message, a
 * `u32`, big-endian, that many bytes of JSON — the provider and the
 * sender — and the body raw to the end; JSON for a count or an error;
 * nothing for a bare answer. A tag that names no variant, or a message
 * shorter than its length, is a decode error.
 */
export function decode(bytes: Uint8Array): Item {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case MESSAGE: {
      if (rest.length < 4) {
        throw new DecodeError("body", tag);
      }
      const length = new DataView(rest.buffer, rest.byteOffset, 4).getUint32(0, false);
      if (rest.length < 4 + length) {
        throw new DecodeError("body", tag);
      }
      const head = Head.parse(jsonOf(rest.subarray(4, 4 + length)));
      return { message: { provider: head.provider, from: head.from, body: rest.subarray(4 + length) } };
    }
    case SKIPPED:
      return { skipped: Skipped.parse(jsonOf(rest)) };
    case FORBIDDEN:
      return "forbidden";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}

/** An item as JSON writes it: the message's body in base64. */
export function json(item: Item): Frame {
  if (typeof item === "string") {
    return item;
  }
  if ("message" in item) {
    return { message: { provider: item.message.provider, from: item.message.from, body: base64(item.message.body) } };
  }
  return item;
}
