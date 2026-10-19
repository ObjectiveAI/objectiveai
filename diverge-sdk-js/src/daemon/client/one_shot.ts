// One request, one answer.

import type { Decode, Encode } from "../../wire/index.js";
import { decodeServerFrame } from "../../wire/frame/server.js";
import type { Handle, Scope } from "../../wire/client/index.js";
import { OneShotError, OpenError } from "./error.js";

/**
 * Encode `request` and open a scope with it. The two ways this
 * fails are the request not encoding and the frame not sending.
 */
export async function open<Q>(handle: Handle, request: Q, encode: Encode<Q>): Promise<Scope> {
  let payload: Uint8Array;
  try {
    payload = encode(request);
  } catch (cause) {
    throw new OpenError("request", cause);
  }
  try {
    return await handle.sendRequest(payload);
  } catch (cause) {
    throw new OpenError("send", cause);
  }
}

/**
 * Send `request` and read its one answer: the response frame,
 * whichever answer it is. The scope's finish follows and is not
 * waited for; the daemon opens no channel on a one-answer scope, so
 * the scope's requests are never read.
 */
export async function execute<Q, A>(handle: Handle, request: Q, encode: Encode<Q>, decode: Decode<A>): Promise<A> {
  let scope: Scope;
  try {
    scope = await open(handle, request, encode);
  } catch (error) {
    throw new OneShotError(error instanceof OpenError ? error.kind : "send", error);
  }
  return answer(scope, decode);
}

/** The one answer on `scope`, read and decoded. */
export async function answer<A>(scope: Scope, decode: Decode<A>): Promise<A> {
  const bytes = await scope.responseReceiver.next();
  if (bytes === undefined) {
    throw new OneShotError("closed");
  }
  let frame;
  try {
    frame = decodeServerFrame(bytes);
  } catch (cause) {
    throw new OneShotError("frame", cause);
  }
  switch (frame.kind) {
    case "response":
      try {
        return decode(frame.payload);
      } catch (cause) {
        throw new OneShotError("response", cause);
      }
    case "response_finish":
      throw new OneShotError("unanswered");
    default:
      throw new OneShotError("misrouted");
  }
}
