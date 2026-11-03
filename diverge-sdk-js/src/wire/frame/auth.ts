// The auth frame's payload: the discover flag, and a token.

import { utf8 } from "../encode.js";
import { text } from "../decode.js";
import { FrameError } from "./error.js";

/**
 * The payload of an auth frame, in either direction: the discover
 * flag — one byte, `0` or `1` — then the token, an RFC 7519 JWT, as
 * UTF-8, every byte after the flag, with no length. Each end sends
 * exactly one, the connector's first and the connectee's in answer,
 * from the same issuer. Only a provider honors the flag, listing a
 * peer that sent `1` to its `discover`; this SDK dials daemons, so it
 * sends `0` unless told otherwise.
 */
export function auth(token: string, discover = false): Uint8Array {
  const bytes = utf8(token);
  const payload = new Uint8Array(bytes.length + 1);
  payload[0] = discover ? 1 : 0;
  payload.set(bytes, 1);
  return payload;
}

/**
 * The token an auth frame's payload carries. An empty payload, a flag
 * that is neither `0` nor `1`, an empty token, or a token that is not
 * UTF-8 is malformed: the `auth` error, and the reader closes the
 * connection. The flag is read and ignored, since only a provider
 * honors it.
 */
export function readAuth(payload: Uint8Array): string {
  const flag = payload[0];
  if (payload.length < 2 || (flag !== 0 && flag !== 1)) {
    throw new FrameError("auth");
  }
  try {
    return text(payload.subarray(1));
  } catch (cause) {
    throw new FrameError("auth", undefined, cause);
  }
}
