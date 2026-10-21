// One channel, one answer.

import { decodeServerFrame } from "../../../../wire/frame/server.js";
import type { Handle } from "../../../../wire/client/handle.js";
import type { Answered } from "./answered.js";
import { UnaryError } from "./error.js";

/**
 * Open a channel on `scope` with `payload` and read its one answer:
 * the first frame on it, a channel response, read by `answered`; a
 * finish before any answer is `unanswered`; a refusal is the
 * `refused` error carrying it. What follows the answer — the finish
 * — is left unread.
 */
export async function unary<T, R>(handle: Handle, scope: number, payload: Uint8Array, answered: Answered<T, R>): Promise<T> {
  let channel;
  try {
    channel = await handle.sendChannelRequest(scope, payload);
  } catch (cause) {
    throw new UnaryError<R>("send", cause);
  }
  const bytes = await channel.responseReceiver.next();
  if (bytes === undefined) {
    throw new UnaryError<R>("closed");
  }
  let frame;
  try {
    frame = decodeServerFrame(bytes);
  } catch (cause) {
    throw new UnaryError<R>("frame", cause);
  }
  switch (frame.kind) {
    case "channel_response":
      break;
    case "channel_response_finish":
      throw new UnaryError<R>("unanswered");
    default:
      throw new UnaryError<R>("misrouted");
  }
  let answer;
  try {
    answer = answered(frame.payload);
  } catch (cause) {
    throw new UnaryError<R>("response", cause);
  }
  if ("refusal" in answer) {
    throw new UnaryError<R>("refused", undefined, answer.refusal);
  }
  return answer.item;
}
