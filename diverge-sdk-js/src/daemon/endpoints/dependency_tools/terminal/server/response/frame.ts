// A terminal's answer: its output.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";
import { base64 } from "../../../../../../shared/containers/exec/response/base64.js";

/**
 * A terminal's answer, as JSON writes it: one read of the terminal —
 * stdout and stderr together — its bytes in standard base64; how the
 * shell ended; or nothing is the one named, forbidden, or a failure.
 * What the stream yields is the `Item`, the same frame with the bytes.
 * A stream: any number of `stdout`, then exactly one `exited` or
 * `signaled`, then the finish; or exactly one `not_found`, `forbidden`
 * or `error`, then the finish.
 */
export const Frame = z.union([
  /** One read of the terminal. Tag `0`. */
  z.strictObject({ stdout: z.base64() }),
  /** The shell exited, with this code. Last. Tag `1`. */
  z.strictObject({ exited: z.number().int() }),
  /** A signal ended it, this one. Last. Tag `2`. */
  z.strictObject({ signaled: z.number().int() }),
  /** Nothing is the one named — or, for a dependency tool, none runs. Tag `3`. */
  z.literal("not_found"),
  /** The account holds no `exec` grant reaching it. Tag `4`. */
  z.literal("forbidden"),
  /** No shell started, or the daemon failed. Tag `5`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

/** One answer as the stream yields it: the frame with stdout's bytes. */
export type Item =
  | { readonly stdout: Uint8Array }
  | { readonly exited: number }
  | { readonly signaled: number }
  | "not_found"
  | "forbidden"
  | { readonly error: Error };

const STDOUT = 0;
const EXITED = 1;
const SIGNALED = 2;
const NOT_FOUND = 3;
const FORBIDDEN = 4;
const ERROR = 5;

/**
 * An item out of its payload: the tag byte, then the bytes as they
 * are for stdout, JSON for an exit, a signal or an error, nothing for
 * a bare answer. A tag that names no variant is the `unexpected_tag`
 * error.
 */
export function decode(bytes: Uint8Array): Item {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case STDOUT:
      return { stdout: rest };
    case EXITED:
      return { exited: z.number().int().parse(jsonOf(rest)) };
    case SIGNALED:
      return { signaled: z.number().int().parse(jsonOf(rest)) };
    case NOT_FOUND:
      return "not_found";
    case FORBIDDEN:
      return "forbidden";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}

/** An item as JSON writes it: stdout's bytes in base64. */
export function json(item: Item): Frame {
  if (typeof item === "string") {
    return item;
  }
  if ("stdout" in item) {
    return { stdout: base64(item.stdout) };
  }
  return item;
}
