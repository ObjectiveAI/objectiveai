// The auth frame's payload: a token.

import { utf8 } from "../encode.js";
import { text } from "../decode.js";
import { FrameError } from "./error.js";

/**
 * The payload of an auth frame, in either direction: the token, an
 * RFC 7519 JWT, as UTF-8 — the whole payload, with no mode byte and
 * no length. Each end sends exactly one, the connector's first and
 * the connectee's in answer, from the same issuer.
 */
export function auth(token: string): Uint8Array {
  return utf8(token);
}

/**
 * The token an auth frame's payload is. An empty payload, or one that
 * is not UTF-8, is malformed: the `auth` error, and the reader closes
 * the connection.
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
