// The one channel a client opens on a message scope: the cancel.

/**
 * What a client asks the daemon for while a message is open: one
 * thing, `cancel`, tag `0`, carrying nothing. Nothing comes back on this
 * channel; what comes back is the scope's own answer: a cancel in time
 * ends the scope with the finish and no response before it, and a
 * cancel after the receiver had the message changes nothing.
 */
export type Frame = "cancel";

/** Tag for `cancel`. */
const CANCEL = 0;

/** One byte, laid out by hand: one tag is not a shape a format would help with. */
export function encode(_frame: Frame): Uint8Array {
  return Uint8Array.of(CANCEL);
}
