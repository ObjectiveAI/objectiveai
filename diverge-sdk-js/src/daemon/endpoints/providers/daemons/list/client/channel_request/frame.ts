// The one channel a client opens on a list scope: the cancel.

/**
 * What a client asks the daemon for while a list is open: one thing,
 * `cancel`, tag `0`, carrying nothing. Nothing comes back on this
 * channel — the daemon sends no channel response and no finish on it;
 * what comes back is the scope's finish: the daemon stops listing
 * and finishes the scope, and frames it had already sent precede the
 * finish, since a cancel is a request and not a recall. A cancel of
 * a list that has already finished changes nothing.
 */
export type Frame = "cancel";

/** Tag for `cancel`. */
const CANCEL = 0;

/** One byte, laid out by hand: one tag is not a shape a format would help with. */
export function encode(_frame: Frame): Uint8Array {
  return Uint8Array.of(CANCEL);
}
