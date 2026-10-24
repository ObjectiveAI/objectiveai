// The wire: what every protocol in this package is spoken over.
//
// One WebSocket, every message one BINARY frame with a nine-byte
// header — `frame`; how a type becomes a payload's bytes and comes
// back out — `encode`, `decode` and `postcard`; the socket itself —
// `connection`; the frame-level `client`, which mints scopes and
// channels and routes what answers them; and `proxy`, the same
// client held through a Tauri app's Rust. None of it names an
// endpoint: the daemon defines its own request vocabulary on top of
// this and nothing here knows which it is carrying.

export * as frame from "./frame/index.js";
export * as client from "./client/index.js";
export * as proxy from "./proxy/index.js";
export * as postcard from "./postcard.js";
export * from "./encode.js";
export * from "./decode.js";
export * from "./connection.js";
