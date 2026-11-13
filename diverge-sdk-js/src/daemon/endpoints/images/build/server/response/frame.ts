// A build's answers.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";
import { Built } from "./built.js";
import { Exists } from "./exists.js";

/**
 * A line the build said, or the one answer that ends it.
 *
 * A build is a stream with an end: zero or more `{ output }` frames,
 * each a line the provider's build tool wrote, as it wrote them;
 * then exactly one of the others; then the finish.
 *
 * 1. Judged first: `forbidden` and `not_found` come with no output
 *    before them, since nothing was asked of the provider.
 * 2. Then the provider's own answers, relayed: `{ built }`,
 *    `{ exists }`, and the three refusals, after each of which
 *    nothing exists.
 * 3. Or a failure, `{ error }`: the provider is not connected, the
 *    build failed, or the exchange broke. The output before it says
 *    what the build said.
 */
export const Frame = z.union([
  /** A line of the build's output, as the build tool wrote it. Tag `0`. */
  z.strictObject({ output: z.string() }),
  /** The image is in the daemon's Store on the provider. Tag `1`. */
  z.strictObject({ built: Built }),
  /** The build produced a digest owned already, and nothing was added. Tag `2`. */
  z.strictObject({ exists: Exists }),
  /** The Store has no room for the image, and nothing exists. Tag `3`. */
  z.literal("insufficient_capacity"),
  /** Something holds the volume the context names, and nothing exists. Tag `4`. */
  z.literal("context_held"),
  /** The provider has no Store, and nothing exists. Tag `5`. */
  z.literal("unsupported"),
  /** The caller's grants do not allow it: no `build` over images, or no `mount` over the context volume. Tag `6`. */
  z.literal("forbidden"),
  /** The provider is none the daemon keeps a record of, or the context volume is none the provider lists. Tag `7`. */
  z.literal("not_found"),
  /** A failure, in the daemon's words or the provider's. Tag `8`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const OUTPUT = 0;
const BUILT = 1;
const EXISTS = 2;
const INSUFFICIENT_CAPACITY = 3;
const CONTEXT_HELD = 4;
const UNSUPPORTED = 5;
const FORBIDDEN = 6;
const NOT_FOUND = 7;
const ERROR = 8;

/**
 * A frame out of its payload: the tag byte, then the variant's own
 * JSON, parsed as what the tag promised — nothing for a bare answer.
 * A tag that names no variant is the `unexpected_tag` error.
 */
export function decode(bytes: Uint8Array): Frame {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case OUTPUT:
      return { output: z.string().parse(jsonOf(rest)) };
    case BUILT:
      return { built: Built.parse(jsonOf(rest)) };
    case EXISTS:
      return { exists: Exists.parse(jsonOf(rest)) };
    case INSUFFICIENT_CAPACITY:
      return "insufficient_capacity";
    case CONTEXT_HELD:
      return "context_held";
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
