//! The client half, held through an app's native side.
//!
//! A page in a Tauri app holds no socket. Its native side — Rust, in
//! the app — dials, makes the handshake and keeps the one
//! [`Handle`](crate::wire::client::handle::Handle); the page asks it
//! to send through four commands and receives what answers on the
//! app's channels. The TypeScript SDK's proxy handle is the page's
//! side of that; this module is what the native side needs to be the
//! other, so that nothing above the handle can tell a proxied
//! connection from a dialled one. Nothing here names Tauri: what
//! crosses the boundary is stated in terms of frames and closures,
//! and the app supplies the channel.
//!
//! # The contract
//!
//! - **The four commands.** `wire_request(payload, responses,
//!   requests) -> scope` calls
//!   [`send_request`](crate::wire::client::handle::Handle::send_request)
//!   and answers the scope's number; `wire_channel_request(scope,
//!   payload, responses) -> channel` calls
//!   [`send_channel_request`](crate::wire::client::handle::Handle::send_channel_request)
//!   and answers the channel's number; `wire_channel_response(scope,
//!   channel, payload)` and `wire_channel_response_finish(scope,
//!   channel)` are the two sends. Every payload is written as given.
//! - **A channel carries whole frames, then the end.** Each channel the
//!   page hands a command carries the frames of exactly one receiver
//!   — the scope's responses, the scope's requests, or a channel's
//!   responses — one whole frame per message, header included, in the
//!   order the receiver yielded them, and then exactly one EMPTY
//!   message, the end. No message follows the end. A frame is never
//!   empty, since its header is nine bytes, so the end is told from a
//!   frame by its length alone. The end is sent when the receiver
//!   closes, whatever closed it: the router forwarding a finish, a
//!   scope ending, or the connection ending. [`forward()`] does this.
//! - **A refusal.** A command that cannot send rejects with a
//!   [`Refusal`], serialized as
//!   `{"kind": "connection" | "scope", "message": "..."}`:
//!   [`SendError`](crate::wire::client::handle::SendError) in the two
//!   kinds the page's own send error has. `scope` is the one scope
//!   not being open; everything else — the socket gone, the router
//!   gone, no connection, a dial that failed — is `connection`.
//! - **A connection that ended stays ended.** Every send on it is
//!   refused `connection` until the app dials again. No send dials on
//!   the page's behalf; dialling is asked for, as
//!   [`dial`](crate::wire::client::dial::dial) is a socket client's.
//! - **The link's two commands.** `wire_status() -> WireStatus` dials
//!   when nothing has been dialled since the link was made or reset,
//!   and answers how the link stands — [`WireStatus`]; a link that is
//!   down stays down. `wire_reset()` puts the link back to not yet
//!   dialled, so the next use dials. Neither is refused. [`Link`]
//!   keeps these rules; the Tauri commands that state them are
//!   `crate::tauri_proxy`, with the `tauri-proxy`
//!   feature.
//!
//! What this buys is parity: over a socket, the connection ending
//! closes every receiver, and so every open stream fails with
//! `closed` and every later send with `connection`; through the
//! proxy, the same end reaches the page as each channel's end and
//! each command's refusal, and the page's code is the same code.

pub mod forward;
pub mod link;
pub mod refusal;
pub mod status;

pub use forward::forward;
pub use link::Link;
pub use refusal::Refusal;
pub use status::WireStatus;
