// The frame-level client, held through a Tauri app's Rust.
//
// The app's Rust side holds the authenticated connection and its own
// `diverge_sdk::wire::client::Handle`; this side asks it, frame by
// frame, through four commands, and receives frames WHOLE on the
// app's channels. Numbers are minted by Rust and handed back. Above
// the handle, nothing differs from the socket half: the same scope,
// the same receivers of the same frames. The webview never holds an
// address or a credential.

export * from "./transport.js";
export * from "./commands.js";
export * from "./handle.js";
