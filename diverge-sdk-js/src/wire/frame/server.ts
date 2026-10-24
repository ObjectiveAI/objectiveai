// The frames a server sends, as a client reads them.

import { AUTH, CHANNEL_REQUEST, CHANNEL_RESPONSE, CHANNEL_RESPONSE_FINISH, RESPONSE, RESPONSE_FINISH } from "./kind.js";
import { splitHeader } from "./header.js";
import { FrameError } from "./error.js";

/**
 * One frame as a server sends it: its token, the handshake's
 * answer, which a client never expects after it; a response or the finish on a
 * scope the client opened; a channel request the server opens
 * inside that scope; or a response or finish on a channel the
 * CLIENT opened. A `request` from a server is a type this side does
 * not read, and decoding one is the `unknown_type` error, as is any
 * byte past six.
 */
export type ServerFrame =
  | { readonly kind: "auth"; readonly payload: Uint8Array }
  | { readonly kind: "response"; readonly scope: number; readonly payload: Uint8Array }
  | { readonly kind: "response_finish"; readonly scope: number }
  | { readonly kind: "channel_request"; readonly scope: number; readonly channel: number; readonly payload: Uint8Array }
  | { readonly kind: "channel_response"; readonly scope: number; readonly channel: number; readonly payload: Uint8Array }
  | { readonly kind: "channel_response_finish"; readonly scope: number; readonly channel: number };

/**
 * Read one server frame out of a whole message. On a scope-level
 * frame the channel is meaningless, so a non-zero one is ignored
 * rather than rejected.
 */
export function decodeServerFrame(bytes: Uint8Array): ServerFrame {
  const { type, scope, channel, payload } = splitHeader(bytes);
  switch (type) {
    case AUTH:
      return { kind: "auth", payload };
    case RESPONSE:
      return { kind: "response", scope, payload };
    case RESPONSE_FINISH:
      return { kind: "response_finish", scope };
    case CHANNEL_REQUEST:
      return { kind: "channel_request", scope, channel, payload };
    case CHANNEL_RESPONSE:
      return { kind: "channel_response", scope, channel, payload };
    case CHANNEL_RESPONSE_FINISH:
      return { kind: "channel_response_finish", scope, channel };
    default:
      throw new FrameError("unknown_type", type);
  }
}
