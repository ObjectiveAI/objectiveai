// The one channel a client opens on an exec: the cancel.

/**
 * What a client asks the daemon for while an exec runs: one thing,
 * `cancel`, tag `0`, carrying nothing, which kills the command; the
 * scope's finish answers it.
 */
export type Frame = "cancel";

/** Tag for `cancel`. */
const CANCEL = 0;

/** One byte, laid out by hand. */
export function encode(_frame: Frame): Uint8Array {
  return Uint8Array.of(CANCEL);
}
