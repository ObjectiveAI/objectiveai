// The auth frame's payload: the discover flag, a description with it, and a token.

import { utf8 } from "../encode.js";
import { text } from "../decode.js";
import { FrameError } from "./error.js";

/**
 * The payload of an auth frame, in either direction: the discover
 * flag — one byte, `0`, `1` or `2` — then, after `2` only, the
 * description's byte length as a `u16`, big-endian, and its UTF-8
 * bytes; then the token, an RFC 7519 JWT, as UTF-8, every byte after
 * those, with no length. Each end sends exactly one, the connector's
 * first and the connectee's in answer, from the same issuer. Only a
 * provider honors the flag, listing a peer that sent `1` or `2` to its
 * `discover`; this SDK dials daemons, so it sends `0` unless told
 * otherwise, and never a description.
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
 * that is none of `0`, `1` and `2`, a description that runs past the
 * payload, an empty token, or a token that is not UTF-8 is malformed:
 * the `auth` error, and the reader closes the connection. The flag and
 * the description are read past and ignored, since only a provider
 * honors them.
 */
export function readAuth(payload: Uint8Array): string {
  const flag = payload[0];
  let start = 1;
  if (flag === 2) {
    if (payload.length < 3) {
      throw new FrameError("auth");
    }
    start = 3 + ((payload[1]! << 8) | payload[2]!);
  } else if (flag !== 0 && flag !== 1) {
    throw new FrameError("auth");
  }
  if (payload.length <= start) {
    throw new FrameError("auth");
  }
  try {
    return text(payload.subarray(start));
  } catch (cause) {
    throw new FrameError("auth", undefined, cause);
  }
}
