// A message's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Modality } from "../../../../../../shared/containers/info/modality.js";
import { Error } from "../../../../../../shared/error.js";

/**
 * A message's answer: delivered, forbidden, a failure, or the refusal
 * the agent's image's document makes of it. Exactly one per scope,
 * when the fate is known, before the finish. A message refused by the
 * document reached no provider and no container, and the agent's log
 * has no line of it.
 */
export const Frame = z.union([
  /** The agent's program accepted the message: it is in the program's own queue, or in the run it was started with. Tag `0`. */
  z.literal("delivered"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `1`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `2`. */
  z.strictObject({ error: Error }),
  /** The agent's image does not chat: its document states `chat` as `false`, or leaves it out. Tag `3`. */
  z.literal("no_chat"),
  /**
   * The message carries what the agent's image does not take: each
   * modality once, in the order image, audio, file, never empty. The
   * whole message is refused. Tag `4`.
   */
  z.strictObject({ unsupported_modalities: z.array(Modality) }),
]);
export type Frame = z.infer<typeof Frame>;

const DELIVERED = 0;
const FORBIDDEN = 1;
const ERROR = 2;
const NO_CHAT = 3;
const UNSUPPORTED_MODALITIES = 4;

/**
 * A frame out of its payload: the tag byte, then the variant's own
 * JSON, parsed as what the tag promised — nothing for a bare
 * answer, the error's value for a failure, the modalities for a
 * message that carries too much. A tag that names no
 * variant is the `unexpected_tag` error.
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
      return { unsupported_modalities: z.array(Modality).parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
