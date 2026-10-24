// A list's answer: a stream kept open.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";
import type { Change } from "../../../../../../shared/listing/change.js";
import { Event } from "../../../../../../daemon/endpoints/events/event.js";

/**
 * A list's answer: an event added or changed, the word that the list
 * is whole, forbidden, or a failure. First the latest `count` events
 * that match, in id order, each as `added`; then exactly one
 * `listed`; then, for the scope's life and with no cap, an event come
 * to be — or one the window left out, now changed — as `added`, and
 * one sent already whose count has been raised as `changed`. There is
 * no `removed`: an edge of the graph is never taken away.
 */
export const Frame = z.union([
  /** One event that matches, listed now and not before. Tag `0`. */
  z.strictObject({ added: Event }),
  /** One event listed already, as it is now: its count raised since it was last sent. Tag `1`. */
  z.strictObject({ changed: Event }),
  /** Every event in the window when the scope opened has been sent. Tag `2`. */
  z.literal("listed"),
  /** The account the request is served for holds no grant allowing it; nothing was sent. Tag `3`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `4`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const ADDED = 0;
const CHANGED = 1;
const LISTED = 2;
const FORBIDDEN = 3;
const ERROR = 4;

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
      return { added: Event.parse(jsonOf(rest)) };
    case CHANGED:
      return { changed: Event.parse(jsonOf(rest)) };
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
export function change(frame: Frame): Change<Event> | null {
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
  return null;
}
