// The frames a client sends.

import { AUTH, CHANNEL_REQUEST, CHANNEL_RESPONSE, CHANNEL_RESPONSE_FINISH, REQUEST } from "./kind.js";
import { joinHeader } from "./header.js";

/**
 * One frame as a client sends it. A client sends five of the seven
 * types: its credential, a request that opens a scope, and the three
 * channel frames — a channel it opens inside a scope, and the
 * responses and finish on a channel the SERVER opened, whose number
 * is quoted from the server's request rather than chosen. The
 * payload is written as given, tag and all: this layer does not know
 * what a request says.
 */
export type ClientFrame =
  | { readonly kind: "auth"; readonly payload: Uint8Array }
  | { readonly kind: "request"; readonly scope: number; readonly payload: Uint8Array }
  | { readonly kind: "channel_request"; readonly scope: number; readonly channel: number; readonly payload: Uint8Array }
  | { readonly kind: "channel_response"; readonly scope: number; readonly channel: number; readonly payload: Uint8Array }
  | { readonly kind: "channel_response_finish"; readonly scope: number; readonly channel: number };

/** The frame's bytes: the header, then the payload if it has one. */
export function encodeClientFrame(frame: ClientFrame): Uint8Array {
  switch (frame.kind) {
    case "auth":
      return joinHeader(AUTH, 0, 0, frame.payload);
    case "request":
      return joinHeader(REQUEST, frame.scope, 0, frame.payload);
    case "channel_request":
      return joinHeader(CHANNEL_REQUEST, frame.scope, frame.channel, frame.payload);
    case "channel_response":
      return joinHeader(CHANNEL_RESPONSE, frame.scope, frame.channel, frame.payload);
    case "channel_response_finish":
      return joinHeader(CHANNEL_RESPONSE_FINISH, frame.scope, frame.channel);
  }
}
