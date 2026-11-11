// A providers delete's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";

/** Exactly one per scope, before the finish. */
export const Frame = z.union([
  /** The record is gone, and its volumes' tags. Tag `0`. */
  z.literal("deleted"),
  /** No record has the identity; nothing changed. Tag `1`. */
  z.literal("not_found"),
  /** The provider is enabled: disable it first; nothing changed. Tag `2`. */
  z.literal("enabled"),
  /** An agent or a tool is pinned to the provider or mounts one of its volumes; nothing changed. Tag `3`. */
  z.literal("in_use"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `4`. */
  z.literal("forbidden"),
  /** A failure; nothing changed. See `shared.error.Error` for why it says so little. Tag `5`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

/** A frame out of its payload: the tag byte, and the error's JSON for a failure. */
export function decode(bytes: Uint8Array): Frame {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case 0:
      return "deleted";
    case 1:
      return "not_found";
    case 2:
      return "enabled";
    case 3:
      return "in_use";
    case 4:
      return "forbidden";
    case 5:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
