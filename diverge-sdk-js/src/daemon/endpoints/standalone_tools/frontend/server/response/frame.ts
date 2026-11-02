// A frontend request's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";
import { base64 } from "../../../../../../shared/containers/exec/response/base64.js";
import { Head } from "../../../../../../shared/containers/frontend/output/head.js";

/**
 * A frontend request's answer, as JSON writes it: the response's head;
 * a piece of its body, its bytes in standard base64; or nothing is the
 * one named, forbidden, no frontend, or a failure. What the stream
 * yields is the `Item`, the same frame with the bytes. A stream:
 * exactly one `head`, then any number of `body`, then the finish; or
 * exactly one `not_found`, `forbidden`, `no_frontend` or `error`, then
 * the finish; an `error` also ends a body broken off.
 */
export const Frame = z.union([
  /** The response's head. First, and once. Tag `0`. */
  z.strictObject({ head: Head }),
  /** A piece of the response's body. Tag `1`. */
  z.strictObject({ body: z.base64() }),
  /** Nothing is the one named — or, for a dependency tool, none runs. Tag `2`. */
  z.literal("not_found"),
  /** The account holds no `frontend` grant reaching it. Tag `3`. */
  z.literal("forbidden"),
  /** The tool's image declares no frontend. Tag `4`. */
  z.literal("no_frontend"),
  /** No answer, or the daemon failed. Tag `5`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

/** One answer as the stream yields it: the frame with the body's bytes. */
export type Item =
  | { readonly head: Head }
  | { readonly body: Uint8Array }
  | "not_found"
  | "forbidden"
  | "no_frontend"
  | { readonly error: Error };

const HEAD = 0;
const BODY = 1;
const NOT_FOUND = 2;
const FORBIDDEN = 3;
const NO_FRONTEND = 4;
const ERROR = 5;

/**
 * An item out of its payload: the tag byte, then JSON for a head or an
 * error, the bytes as they are for a body, nothing for a bare answer. A
 * tag that names no variant is the `unexpected_tag` error.
 */
export function decode(bytes: Uint8Array): Item {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case HEAD:
      return { head: Head.parse(jsonOf(rest)) };
    case BODY:
      return { body: rest };
    case NOT_FOUND:
      return "not_found";
    case FORBIDDEN:
      return "forbidden";
    case NO_FRONTEND:
      return "no_frontend";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}

/** An item as JSON writes it: the body's bytes in base64. */
export function json(item: Item): Frame {
  if (typeof item === "string") {
    return item;
  }
  if ("body" in item) {
    return { body: base64(item.body) };
  }
  return item;
}
