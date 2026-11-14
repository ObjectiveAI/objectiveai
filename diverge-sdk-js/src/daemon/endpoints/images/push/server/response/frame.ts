// A push's answers.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";
import { Blob } from "./blob.js";
import { Exists } from "./exists.js";
import { Pushed } from "./pushed.js";

/**
 * A blob pulled, or the one answer that ends the push.
 *
 * A push is a stream with an end: zero or more `{ blob }` frames, one
 * per blob the provider pulled from the client; then exactly one of
 * the others; then the finish.
 *
 * 1. Judged first: `forbidden` and `not_found` come with nothing
 *    before them, since nothing was asked of the provider.
 * 2. Then the provider's own answers, relayed: `{ pushed }`,
 *    `{ exists }`, and the five refusals, after each of which nothing
 *    is owned.
 * 3. Or a failure, `{ error }`: the provider is not connected, or the
 *    exchange broke.
 */
export const Frame = z.union([
  /** A blob pulled from the client and verified. Tag `0`. */
  z.strictObject({ blob: Blob }),
  /** The image is in the daemon's Store on the provider. Tag `1`. */
  z.strictObject({ pushed: Pushed }),
  /** The daemon already owns the digest there, and nothing moved. Tag `2`. */
  z.strictObject({ exists: Exists }),
  /** The Store has no room for the image, by the sizes its manifest declares. Tag `3`. */
  z.literal("insufficient_capacity"),
  /** No registry the provider uses serves the image, and the client did not supply all of it. Tag `4`. */
  z.literal("unavailable"),
  /** The client supplied the image and did not prove it holds it. Tag `5`. */
  z.literal("proof_failed"),
  /** The image is not one the provider takes. Tag `6`. */
  z.literal("malformed"),
  /** The provider has no Store, and nothing moved. Tag `7`. */
  z.literal("unsupported"),
  /** The caller's grants do not allow it: no `push` over images reaching the provider. Tag `8`. */
  z.literal("forbidden"),
  /** The provider is none the daemon keeps a record of. Tag `9`. */
  z.literal("not_found"),
  /** A failure, in the daemon's words or the provider's. Tag `10`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const BLOB = 0;
const PUSHED = 1;
const EXISTS = 2;
const INSUFFICIENT_CAPACITY = 3;
const UNAVAILABLE = 4;
const PROOF_FAILED = 5;
const MALFORMED = 6;
const UNSUPPORTED = 7;
const FORBIDDEN = 8;
const NOT_FOUND = 9;
const ERROR = 10;

/**
 * A frame out of its payload: the tag byte, then the variant's own
 * JSON, parsed as what the tag promised — nothing for a bare answer.
 * A tag that names no variant is the `unexpected_tag` error.
 */
export function decode(bytes: Uint8Array): Frame {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case BLOB:
      return { blob: Blob.parse(jsonOf(rest)) };
    case PUSHED:
      return { pushed: Pushed.parse(jsonOf(rest)) };
    case EXISTS:
      return { exists: Exists.parse(jsonOf(rest)) };
    case INSUFFICIENT_CAPACITY:
      return "insufficient_capacity";
    case UNAVAILABLE:
      return "unavailable";
    case PROOF_FAILED:
      return "proof_failed";
    case MALFORMED:
      return "malformed";
    case UNSUPPORTED:
      return "unsupported";
    case FORBIDDEN:
      return "forbidden";
    case NOT_FOUND:
      return "not_found";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
