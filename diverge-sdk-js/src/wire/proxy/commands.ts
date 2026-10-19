// The four commands the app's Rust implements.
//
// Each is a thin call on the Rust side's handle, and the contract is
// the handle's own: `sendRequest`, `sendChannelRequest`,
// `sendChannelResponse`, `sendChannelResponseFinish`. Payloads cross
// as byte arrays; every message on a channel is one whole wire
// frame, header included, exactly what the socket half's receivers
// carry.

/** `(payload, responses, requests) -> scope`: open a scope. */
export const WIRE_REQUEST = "wire_request";
/** `(scope, payload, responses) -> channel`: open a channel in it. */
export const WIRE_CHANNEL_REQUEST = "wire_channel_request";
/** `(scope, channel, payload)`: answer a channel the daemon opened. */
export const WIRE_CHANNEL_RESPONSE = "wire_channel_response";
/** `(scope, channel)`: end that answer. */
export const WIRE_CHANNEL_RESPONSE_FINISH = "wire_channel_response_finish";

/** What a frame looks like after Tauri's serialization, normalized. */
export function bytesOf(message: unknown): Uint8Array {
  if (message instanceof Uint8Array) {
    return message;
  }
  if (message instanceof ArrayBuffer) {
    return new Uint8Array(message);
  }
  if (Array.isArray(message)) {
    return Uint8Array.from(message as number[]);
  }
  throw new TypeError("a proxy channel message that is not bytes");
}
