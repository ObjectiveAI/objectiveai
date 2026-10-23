// The contract between the page and the app's Rust.
//
// The app's Rust holds the authenticated connection and its own
// `diverge_sdk::wire::client::Handle`. The page asks it to send
// through four commands and receives what answers on the app's
// channels. The Rust SDK's `wire::proxy` states the same contract
// from its side and supplies the pieces that keep it.
//
// THE FOUR COMMANDS. `wire_request(payload, responses, requests)`
// opens a scope and answers its number; `wire_channel_request(scope,
// payload, responses)` opens a channel in it and answers the
// channel's number; `wire_channel_response(scope, channel, payload)`
// answers a channel the daemon opened; `wire_channel_response_finish
// (scope, channel)` ends that answer. Each is one thin call on the
// Rust handle, and every payload is written as given. The page
// attaches `onmessage` to every channel before it invokes the
// command that opens it, since a channel message and the invoke's
// settlement are unordered across the boundary.
//
// A CHANNEL CARRIES WHOLE FRAMES, THEN THE END. Each channel the page
// hands a command carries the frames of exactly one receiver on the
// Rust side — the scope's responses, the scope's requests, or a
// channel's responses — one whole wire frame per message, header
// included, in the order the receiver yielded them, and then exactly
// one EMPTY message, the end. No message follows the end. A frame is
// never empty, since its header is nine bytes, so the end is told
// from a frame by its length alone. Rust sends the end when its
// receiver closes, whatever closed it: its router forwarding a
// finish, the scope ending, or the connection ending. The page
// closes its receiver on the end and on nothing else; a finish frame
// is delivered as a frame like any other, and the end follows it.
//
// A REFUSAL. A command that cannot send rejects with a `Refusal`:
// `{ "kind": "connection" | "scope", "message": string }`, the Rust
// handle's send error in this package's own two kinds. `scope` is
// the one scope not being open; everything else — the socket gone,
// the router gone, nothing dialled, a dial that failed — is
// `connection`. The handle throws `SendError(kind, cause)`.
//
// A CONNECTION THAT ENDED STAYS ENDED. Every send on it is refused
// `connection` until the app dials again; no command dials on the
// page's behalf. Dialling is the app's own act, as `wire.client.dial`
// is a socket client's.
//
// What the contract buys is parity: over a socket, the connection
// ending closes every receiver, so every open stream fails with
// `closed` and every later send with `connection`; through the
// proxy, the same end reaches the page as each channel's end and
// each command's refusal, and the code above the handle is the same
// code.

import type { SendErrorKind } from "../client/error.js";

/** `(payload, responses, requests) -> scope`: open a scope. */
export const WIRE_REQUEST = "wire_request";
/** `(scope, payload, responses) -> channel`: open a channel in it. */
export const WIRE_CHANNEL_REQUEST = "wire_channel_request";
/** `(scope, channel, payload)`: answer a channel the daemon opened. */
export const WIRE_CHANNEL_RESPONSE = "wire_channel_response";
/** `(scope, channel)`: end that answer. */
export const WIRE_CHANNEL_RESPONSE_FINISH = "wire_channel_response_finish";

/** What a command rejects with: the Rust handle's send error, in this package's two kinds. */
export interface Refusal {
  /** Which: `scope` for a scope not open, `connection` for everything else. */
  readonly kind: SendErrorKind;
  /** Why, in words. */
  readonly message: string;
}

/**
 * The kind of a command's rejection: a `Refusal`'s own kind, and
 * `connection` for anything that is not a `Refusal` — the invoke
 * itself failing, the app's Rust not holding the command.
 */
export function refusalKind(cause: unknown): SendErrorKind {
  if (typeof cause === "object" && cause !== null && "kind" in cause && (cause.kind === "connection" || cause.kind === "scope")) {
    return cause.kind;
  }
  return "connection";
}

/** What a frame looks like after Tauri's serialization, normalized; the end is zero bytes. */
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
