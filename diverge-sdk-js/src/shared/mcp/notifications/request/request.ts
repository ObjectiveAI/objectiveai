// What a `notifications` asks.


/**
 * The ask for everything the server says on its own account: it
 * carries nothing — the channel's opening is the subscription — and
 * is a type all the same, so that a frame naming five exchanges
 * names them the same way.
 */
export type Request = null;

/** Nothing: the channel's opening is the whole of the ask. */
export function encode(_request: Request): Uint8Array {
  return new Uint8Array(0);
}
