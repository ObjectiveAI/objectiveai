// A structured decision's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../../wire/decode.js";
import { Lacking } from "../../../../../../../shared/containers/info/structured_decisions.js";
import { Error } from "../../../../../../../shared/error.js";

/**
 * A structured decision's answer: delivered, forbidden, a failure, or
 * the refusal the agent's image's document makes of it. Not the
 * agent's answers to the questions, which are read in its log.
 * Exactly one per scope, when the fate is known, before the finish. A
 * decision refused by the document or for its questions reached no
 * provider and no container, and the agent's log has no line of it.
 */
export const Frame = z.union([
  /** The agent's program has the decision: the run it starts has begun. Tag `0`. */
  z.literal("delivered"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `1`. */
  z.literal("forbidden"),
  /** A failure. Questions that are not valid are one, of kind `questions`, naming the question. Tag `2`. */
  z.strictObject({ error: Error }),
  /** The agent's image makes no structured decisions: its document states `structured_decisions` as `false`, or leaves it out. Tag `3`. */
  z.literal("no_structured_decisions"),
  /**
   * The decision carries what the agent's image does not take, by
   * where it carries it. At least one of the two lists is not empty.
   * The whole decision is refused. Tag `4`.
   */
  z.strictObject({ unsupported_modalities: Lacking }),
]);
export type Frame = z.infer<typeof Frame>;

const DELIVERED = 0;
const FORBIDDEN = 1;
const ERROR = 2;
const NO_STRUCTURED_DECISIONS = 3;
const UNSUPPORTED_MODALITIES = 4;

/**
 * A frame out of its payload: the tag byte, then the variant's own
 * JSON, parsed as what the tag promised — nothing for a bare answer,
 * the error's value for a failure, the lacking modalities for a
 * decision that carries too much. A tag that names no variant is the
 * `unexpected_tag` error.
 */
export function decode(bytes: Uint8Array): Frame {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case DELIVERED:
      return "delivered";
    case FORBIDDEN:
      return "forbidden";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    case NO_STRUCTURED_DECISIONS:
      return "no_structured_decisions";
    case UNSUPPORTED_MODALITIES:
      return { unsupported_modalities: Lacking.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
