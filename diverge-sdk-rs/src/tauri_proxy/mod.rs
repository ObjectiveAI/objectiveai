//! The Tauri commands that let a page speak to a daemon.
//!
//! A Tauri app whose page uses the TypeScript SDK holds the daemon's
//! connection on its native side and lets the page speak through it:
//! the page's handle is `wire.proxy.handle(transport)`, and every send
//! it makes is one of the commands here. This module is the whole of
//! the native side: the managed state, [`TauriProxy`], and the six
//! commands. It keeps the contract [`wire::proxy`](crate::wire::proxy)
//! states — whole frames, then the end; refusals by kind; a connection
//! that ended stays ended until the page asks for a reset — so over
//! the proxy a page sees a connection exactly as it would over a
//! socket.
//!
//! # Setting it up
//!
//! The app depends on this crate with the `tauri-proxy` feature,
//! manages one [`TauriProxy`], and lists the six commands in its one
//! invoke handler beside its own:
//!
//! ```ignore
//! tauri::Builder::default()
//!     .manage(diverge_sdk::tauri_proxy::TauriProxy::new("ws://127.0.0.1:14980/", &key))
//!     .invoke_handler(tauri::generate_handler![
//!         diverge_sdk::tauri_proxy::wire_request,
//!         diverge_sdk::tauri_proxy::wire_channel_request,
//!         diverge_sdk::tauri_proxy::wire_channel_response,
//!         diverge_sdk::tauri_proxy::wire_channel_response_finish,
//!         diverge_sdk::tauri_proxy::wire_status,
//!         diverge_sdk::tauri_proxy::wire_reset,
//!     ])
//! ```
//!
//! An app's own commands need no capability entry, and these are the
//! app's own once registered. The page writes, once:
//!
//! ```text
//! const transport = { invoke: (c, a) => invoke(c, a), channel: () => new Channel() };
//! const handle = wire.proxy.handle(transport);
//! await wire.proxy.status(transport); // dials, and says how the link stands
//! await wire.proxy.reset(transport);  // after the connection ended: dial again on next use
//! ```
//!
//! The address and the token never reach the page. The token is a
//! [`Token`](crate::identities::Token), read from the host's
//! [`identities`](crate::identities) by the identity the app's
//! configuration names.
//! [`COMMANDS`] names the six, for an app that lists or checks them.

mod commands;
mod proxy;

pub use commands::*;
pub use proxy::*;

/// The six commands, by the names the page invokes them with.
pub const COMMANDS: [&str; 6] = ["wire_request", "wire_channel_request", "wire_channel_response", "wire_channel_response_finish", "wire_status", "wire_reset"];
