// The auth frame's payload: a token, and nothing else.

import { utf8 } from "../encode.js";
import { text } from "../decode.js";
import { FrameError } from "./error.js";

/**
 * The payload of an auth frame, in either direction: the token, an
 * RFC 7519 JWT as UTF-8, every byte of the payload — no flag before
 * it and no length, since WebSocket delimits the message. Each end
 * sends exactly one, the connector's first and the connectee's in
 * answer.
 */
export function auth(token: string): Uint8Array {
  return utf8(token);
}

/**
 * The token an auth frame's payload carries: the whole payload. An
 * empty payload, or one that is not UTF-8, is malformed: the `auth`
 * error, and the reader closes the connection. The token is not
 * parsed here, and not bounded in length.
 */
export function readAuth(payload: Uint8Array): string {
  if (payload.length === 0) {
    throw new FrameError("auth");
  }
  try {
    return text(payload);
  } catch (cause) {
    throw new FrameError("auth", undefined, cause);
  }
}
