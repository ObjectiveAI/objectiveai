// The seven frame types.
//
// | type | client | server |
// |------|--------|--------|
// | 0    | auth | auth |
// | 1    | request | — |
// | 2    | — | response |
// | 3    | — | response finish |
// | 4    | channel request | channel request |
// | 5    | channel response | channel response |
// | 6    | channel response finish | channel response finish |
//
// A number means one thing in both directions, and the blanks are
// what keep it that way: only a client opens a scope, so only a
// client sends 1; only a server answers one, so only a server sends
// 2 and 3. There is no cancel and no acknowledgement of any kind.

/** The credential, first on the socket and never again. */
export const AUTH = 0;
/** A client opening a scope. */
export const REQUEST = 1;
/** A server answering a scope's request. */
export const RESPONSE = 2;
/** A server ending a scope's answer, and with it the scope. */
export const RESPONSE_FINISH = 3;
/** Either side opening a channel inside a scope. */
export const CHANNEL_REQUEST = 4;
/** The channel's responder answering. */
export const CHANNEL_RESPONSE = 5;
/** The channel's responder ending its answer, and the channel. */
export const CHANNEL_RESPONSE_FINISH = 6;
