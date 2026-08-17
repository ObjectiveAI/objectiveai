//! The provider half, behind the `server` feature.
//!
//! Everything else in this crate is a message. This is the part that
//! does something with one, and it is opt-in for exactly that reason:
//! a client, and a tool that only inspects the protocol, should not
//! have to compile a provider to read a frame.
//!
//! # The socket, and only the socket
//!
//! It takes a [`WebSocket`](crate::websocket::WebSocket) — accepted or
//! dialled, because a provider usually waits to be connected to and
//! sometimes connects to a caller it cannot otherwise reach, and
//! answers the same frames either way.
//!
//! What it cannot do is make one. The feature brings `axum` without
//! `http1` or `http2`, and `tokio-tungstenite` with `stream` alone —
//! enough to name both socket types and not to serve or dial with
//! either. A provider stands up its own server, or connects with its
//! own client, and hands over what comes out.
//!
//! Which keeps the choice where it belongs. Where the endpoint lives,
//! what the URL is, what authenticates the upgrade, what else that
//! server or process does — none of it is this protocol's business,
//! and a crate that served HTTP would have opinions about all of it.
//!
//! # Nothing is re-exported
//!
//! A provider that hands over a socket already depends on axum, and
//! naming `axum::extract::ws::WebSocket` through this crate would put
//! a second path on somebody else's type — one that says nothing new
//! and goes stale the day the version underneath it moves. What
//! [`websocket`](crate::websocket) takes, it takes by that name.
//!
//! The [`frame`](crate::frame) layer stays free of both libraries
//! regardless: it decodes `&[u8]`, because axum defines its own
//! `Message` and pins a tungstenite version of its own, and a
//! repository can easily hold two incompatible tungstenites at once —
//! this one does. A provider on some third transport keeps the frame
//! layer and leaves this module off.
//!
//! # A tree of handlers
//!
//! [`Handler`](handler::Handler) is the root: one connection, read to
//! its end. Under it is [`ScopeHandler`](scope_handler::ScopeHandler),
//! one per request, and under that will be channels.
//!
//! Each level knows only about the one below. The connection knows
//! about scopes and does not know what a laboratory is; a scope knows
//! what it was asked for and does not know what a socket is.
//!
//! # Mostly unimplemented, on purpose
//!
//! One path is real: a request nobody can name gets a scope, an error
//! and a finish, because nothing has to be decided to answer it. Every
//! other path reaches an `unimplemented` marked with what it is
//! waiting for — a provider's behaviour, a decision about writing
//! concurrently, or a register of open scopes.
//!
//! Nothing outside this module changes as those land. A message is the
//! same message whether or not somebody compiled the code that answers
//! it.

pub mod handler;
pub mod scope_handler;
