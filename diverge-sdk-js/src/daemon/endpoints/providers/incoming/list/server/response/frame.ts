// A list's answer: a stream kept open.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../../wire/decode.js";
import { Error } from "../../../../../../../shared/error.js";
import type { Change } from "../../../../../../../shared/listing/change.js";
import { Incoming } from "./incoming.js";

/**
 * A list's answer: a credential added, changed or removed, the word that
 * the list is whole, forbidden, or a failure. First every one that
 * matches, oldest first, the first `count` of them, each as `added`;
 * then exactly one `listed`; then, for the scope's life, every
 * change — its record, or where the daemon stands with the provider through it. The finish follows the client's cancel or
 * exactly one `error`; or the scope is exactly one `forbidden`.
 */
export const Frame = z.union([
  /** One credential that matches, listed now and not before. Tag `0`. */
  z.strictObject({ added: Incoming }),
  /** One credential listed already, as it is now, differing from what was last sent. Tag `1`. */
  z.strictObject({ changed: Incoming }),
  /** One credential listed already, matching no more: as it was last sent. Tag `2`. */
  z.strictObject({ removed: Incoming }),
  /** Every credential that matched when the scope opened has been sent. A client that wants the list once cancels here; one that watches reads on. Tag `3`. */
  z.literal("listed"),
  /** The account the request is served for holds no grant allowing it; nothing was sent. Tag `4`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `5`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const ADDED = 0;
const CHANGED = 1;
const REMOVED = 2;
const LISTED = 3;
const FORBIDDEN = 4;
const ERROR = 5;

/**
 * A frame out of its payload: the tag byte, then the variant's own
 * JSON, parsed as what the tag promised — nothing for a bare
 * answer, the error's value for a failure. A tag that names no
 * variant is the `unexpected_tag` error.
 */
export function decode(bytes: Uint8Array): Frame {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case ADDED:
      return { added: Incoming.parse(jsonOf(rest)) };
    case CHANGED:
      return { changed: Incoming.parse(jsonOf(rest)) };
    case REMOVED:
      return { removed: Incoming.parse(jsonOf(rest)) };
    case LISTED:
      return "listed";
    case FORBIDDEN:
      return "forbidden";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}

/**
 * What this frame tells a listing — a `Change`, for `Listing.push` —
 * or `null` where it tells a listing nothing: forbidden, or a
 * failure, the answers that end the scope.
 */
export function change(frame: Frame): Change<Incoming> | null {
  if (frame === "listed") {
    return { kind: "listed" };
  }
  if (frame === "forbidden") {
    return null;
  }
  if ("added" in frame) {
    return { kind: "added", item: frame.added };
  }
  if ("changed" in frame) {
    return { kind: "changed", item: frame.changed };
  }
  if ("removed" in frame) {
    return { kind: "removed", item: frame.removed };
  }
  return null;
}
