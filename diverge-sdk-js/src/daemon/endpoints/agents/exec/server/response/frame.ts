// An exec's answer: the command's stream.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";
import { base64 } from "../../../../../../shared/containers/exec/response/base64.js";

/**
 * An exec's answer, as JSON writes it: one read of stdout or stderr,
 * its bytes in standard base64; how the command ended; or nothing is
 * the one named, forbidden, or a failure. What the stream yields is
 * the `Item`, the same frame with the bytes. A stream: any number of
 * `stdout` and `stderr` in the order the reads completed, then exactly
 * one `exited` or `signaled`, then the finish; or exactly one
 * `not_found`, `forbidden` or `error`, then the finish.
 */
export const Frame = z.union([
  /** One read of the command's stdout. Tag `0`. */
  z.strictObject({ stdout: z.base64() }),
  /** One read of its stderr. Tag `1`. */
  z.strictObject({ stderr: z.base64() }),
  /** It exited, with this code. Last. Tag `2`. */
  z.strictObject({ exited: z.number().int() }),
  /** A signal ended it, this one. Last. Tag `3`. */
  z.strictObject({ signaled: z.number().int() }),
  /** Nothing is the one named — or, for a dependency tool, none runs. Tag `4`. */
  z.literal("not_found"),
  /** The account holds no `exec` grant reaching it. Tag `5`. */
  z.literal("forbidden"),
  /** The command never ran, or the daemon failed. Tag `6`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

/** One answer as the stream yields it: the frame with stdout's and stderr's bytes. */
export type Item =
  | { readonly stdout: Uint8Array }
  | { readonly stderr: Uint8Array }
  | { readonly exited: number }
  | { readonly signaled: number }
  | "not_found"
  | "forbidden"
  | { readonly error: Error };

const STDOUT = 0;
const STDERR = 1;
const EXITED = 2;
const SIGNALED = 3;
const NOT_FOUND = 4;
const FORBIDDEN = 5;
const ERROR = 6;

/**
 * An item out of its payload: the tag byte, then the bytes as they
 * are for stdout and stderr, JSON for an exit, a signal or an error,
 * nothing for a bare answer. A tag that names no variant is the
 * `unexpected_tag` error.
 */
export function decode(bytes: Uint8Array): Item {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case STDOUT:
      return { stdout: rest };
    case STDERR:
      return { stderr: rest };
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

/** An item as JSON writes it: stdout's and stderr's bytes in base64. */
export function json(item: Item): Frame {
  if (typeof item === "string") {
    return item;
  }
  if ("stdout" in item) {
    return { stdout: base64(item.stdout) };
  }
  if ("stderr" in item) {
    return { stderr: base64(item.stderr) };
  }
  return item;
}
