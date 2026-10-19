// The frame-level client: the half that mints scopes and channels.
//
// One socket, authenticated first, then split in two: `Router` is
// the read loop — frames off the socket, forwarded WHOLE to whoever
// registered for them — and `SocketHandle` is the write half and
// what a caller keeps hold of. `dial` makes both. `Handle` is what
// every endpoint takes: the four sends, which the proxy half
// implements too, so that nothing above this line knows which it
// holds.
//
// Every queue here is unbounded. The obligation on a caller is
// therefore sharp: read what you asked for, or drop it.

export * from "./handle.js";
export * from "./scope.js";
export * from "./channel.js";
export * from "./receiver.js";
export * from "./registry.js";
export * from "./router.js";
export * from "./socket_handle.js";
export * from "./authorize.js";
export * from "./dial.js";
export * from "./error.js";
