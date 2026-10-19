// A scope, as the client that opened it holds it.

import type { Receiver } from "./receiver.js";

/**
 * One request's scope: its number, and the two things that arrive
 * on it. `responseReceiver` carries the answer to the request —
 * whole frames, header included, since a finish is a frame and
 * telling one from a response means reading the type — and ends at
 * the finish. `requestReceiver` carries the server asking for
 * something inside this scope: channel requests, whole, whose
 * channel number is the SERVER's and is what an answer quotes. They
 * are separate because a channel number belongs to whoever opened
 * it, so the two cannot share a stream without colliding.
 */
export interface Scope {
  /** The number this request opened. */
  readonly scope: number;
  /** The answer, frame by frame, to the finish. */
  readonly responseReceiver: Receiver<Uint8Array>;
  /** The server's channel requests inside this scope. */
  readonly requestReceiver: Receiver<Uint8Array>;
}
