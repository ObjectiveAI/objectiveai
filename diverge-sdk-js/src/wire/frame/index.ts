// The wire frame.
//
// Every message is one WebSocket BINARY frame:
//
//     [type: u8][scope: u32 big-endian][channel: u32 big-endian][payload…]
//
// `type` leads so a reader learns what a frame is before anything
// else. No length prefix: WebSocket already delimits messages. Nine
// bytes, always.
//
// A SCOPE is one client request and everything that follows from
// it: the client chooses the number and every later frame in either
// direction carries it. A CHANNEL is one exchange inside a scope;
// channel 0 is the answer to the client's own request, and any other
// was opened by a channel request, which BOTH sides may send. The
// numbering is per sender: a client's channel 5 and a server's
// channel 5 are different channels, told apart by the direction a
// frame travelled. A channel carries exactly one request, and a
// stream ends at its finish frame and nowhere else.

export * from "./kind.js";
export * from "./header.js";
export * from "./client.js";
export * from "./server.js";
export * from "./auth.js";
export * from "./error.js";
