// The one channel a client opens on a message: the cancel.

/**
 * What a client asks the daemon for while a message is open: one thing,
 * `cancel`, tag `0`, carrying nothing. Nothing comes back on this
 * channel — the daemon sends no channel response and no finish on it;
 * what comes back is the scope's answer or its finish: the daemon takes the message back out of the agent's inbox if the loop has not taken it, and answers `cancelled`; one the loop took already is `delivered`, and the cancel changed nothing.
 */
export type Frame = "cancel";

/** Tag for `cancel`. */
const CANCEL = 0;

/** One byte, laid out by hand: one tag is not a shape a format would help with. */
export function encode(_frame: Frame): Uint8Array {
  return Uint8Array.of(CANCEL);
}
