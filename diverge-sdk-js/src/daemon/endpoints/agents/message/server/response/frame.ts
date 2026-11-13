// A message's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Entry } from "../../../../logs/server/response/entry.js";
import { Lacking } from "../../../../../../shared/containers/info/structured_decisions.js";
import { Error } from "../../../../../../shared/error.js";

/**
 * A message's answer: its fate, or — watched — what the agent's log
 * took of it.
 *
 * Unwatched: exactly one per scope, when the fate is known, before
 * the finish. A structured decision's answers are not this answer:
 * they are read in the agent's log, or by watching.
 *
 * Watched (`watch: true`), the answer is the part of the agent's log
 * that is the request's, each item one `{ item }` — the same
 * `{ item: { agent: … } }` a `logs` read of the agent gives for it —
 * in the log's order:
 *
 * 1. Nothing before the echo: no `delivered`, and no item the log
 *    took before it.
 * 2. The echo first: a chat message's first user part (`user_*`), a
 *    structured decision's `user_structured_decisions` item.
 * 3. Then every item the log takes after it, whatever it is of, up
 *    to and including the last: for a chat message the first
 *    `inactive` — the end of the run the echo landed in; for a
 *    structured decision the `structured_decisions_response` whose
 *    `id` is the echoed decision's, where one comes, else the first
 *    `inactive`.
 * 4. Then the finish; a cancel, or the agent's deletion, finishes
 *    sooner.
 * 5. A request that reaches no echo has one answer, alone: any of
 *    the unwatched ones save `delivered`, or `dropped`.
 *
 * A request refused by the document or for its questions reached no
 * provider and no container, and the agent's log has no line of it.
 */
export const Frame = z.union([
  /** The agent's program has the request. Never sent to a request that watches. Tag `0`. */
  z.literal("delivered"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `1`. */
  z.literal("forbidden"),
  /** A failure. Questions that are not valid are one, of kind `questions`, naming the question. Tag `2`. */
  z.strictObject({ error: Error }),
  /** The agent's image does not chat: its document states `chat` as `false`, or leaves it out. Tag `3`. */
  z.literal("no_chat"),
  /**
   * The request carries what the agent's image does not take, by
   * where it carries it: its prompt, and — a structured decision's —
   * its choices. At least one of the two lists is not empty; a chat
   * message's `choice` always is. The whole request is refused. Tag `4`.
   */
  z.strictObject({ unsupported_modalities: Lacking }),
  /** The agent's image takes no structured decision: its document states `structured_decisions` as `false`, or leaves it out. Tag `5`. */
  z.literal("no_structured_decisions"),
  /** One item of the agent's log, as a logs read gives it: sent to a request that watches, from its echo on. Tag `6`. */
  z.strictObject({ item: Entry }),
  /**
   * The agent's program accepted the request and its run kept nothing
   * of it: sent to a request that watches, alone. It will not be
   * echoed later: a request belongs to the one run it was delivered
   * into. Tag `7`.
   */
  z.literal("dropped"),
]);
export type Frame = z.infer<typeof Frame>;

const DELIVERED = 0;
const FORBIDDEN = 1;
const ERROR = 2;
const NO_CHAT = 3;
const UNSUPPORTED_MODALITIES = 4;
const NO_STRUCTURED_DECISIONS = 5;
const ITEM = 6;
const DROPPED = 7;

/**
 * A frame out of its payload: the tag byte, then the variant's own
 * JSON, parsed as what the tag promised — nothing for a bare answer,
 * the error's value for a failure, the lacking modalities for a
 * request that carries too much, the log's item for an item. A tag
 * that names no variant is the `unexpected_tag` error.
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
    case NO_CHAT:
      return "no_chat";
    case UNSUPPORTED_MODALITIES:
      return { unsupported_modalities: Lacking.parse(jsonOf(rest)) };
    case NO_STRUCTURED_DECISIONS:
      return "no_structured_decisions";
    case ITEM:
      return { item: Entry.parse(jsonOf(rest)) };
    case DROPPED:
      return "dropped";
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
