//! The one connection to the daemon, and the page's way onto it.
//!
//! Rust dials the daemon `viewer.yaml` names, as the account whose key
//! it holds, and keeps the one [`Handle`](diverge_sdk::wire::client::handle::Handle);
//! the page never sees the address or the key. The page speaks the
//! daemon's protocol itself, with the TypeScript SDK, whose proxy
//! handle is four commands — [`wire_request`], [`wire_channel_request`],
//! [`wire_channel_response`], [`wire_channel_response_finish`] — each a
//! thin call on the handle, every frame crossing whole, header
//! included, as bytes on a Tauri channel, and every channel ending
//! with one empty message when the handle's receiver closes; a send
//! that cannot go rejects with the SDK's
//! [`Refusal`](diverge_sdk::wire::proxy::Refusal). The contract is
//! the SDK's, in [`diverge_sdk::wire::proxy`]; this module keeps it.
//! Two more commands are the app's own: [`wire_status`], which dials
//! on first use and says how the link stands, and [`wire_reset`],
//! which puts the link back to not yet dialled, so the next use dials.
//! A connection that ended stays ended until then — see [`Wire`].

mod commands;
mod dial;
mod error;
mod link;
mod refusing;
mod status;

pub use commands::*;
pub use dial::*;
pub use error::*;
pub use link::*;
pub use refusing::*;
pub use status::*;
