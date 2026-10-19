// A channel, as the client that opened it holds it.

import type { Receiver } from "./receiver.js";

/**
 * One channel this end opened inside a scope: its number, in this
 * end's numbering, and the one thing coming back on it — the
 * responder's answer, whole frames to the finish.
 */
export interface Channel {
  /** The number this end minted for it. */
  readonly channel: number;
  /** The answer, frame by frame, to the finish. */
  readonly responseReceiver: Receiver<Uint8Array>;
}
