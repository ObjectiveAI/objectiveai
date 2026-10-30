// The one channel a client opens on a log: the cancel.

/**
 * What a client asks the daemon for while a log is open: one thing,
 * `cancel`, tag `0`, carrying nothing. Nothing comes back on this
 * channel — the daemon sends no channel response and no finish on it;
 * what comes back is the scope's answer or its finish: the daemon stops sending and finishes the scope, and the items sent before stand.
 */
export type Frame = "cancel";

/** Tag for `cancel`. */
const CANCEL = 0;

/** One byte, laid out by hand: one tag is not a shape a format would help with. */
export function encode(_frame: Frame): Uint8Array {
  return Uint8Array.of(CANCEL);
}
