//! The provider half, behind the `server` feature.
//!
//! Everything else in this crate is a message. This is the part that
//! does something with one, and it is opt-in for exactly that reason:
//! a client, and a tool that only inspects the protocol, should not
//! have to compile a provider to read a frame.
//!
//! # The socket, and only the socket
//!
//! It takes a [`Connection`](crate::connection::Connection) — incoming or
//! outgoing, because a provider usually waits to be connected to and
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
//! [`connection`](crate::connection) takes, it takes by that name.
//!
//! The [`frame`](crate::frame) layer stays free of both libraries
//! regardless: it decodes `&[u8]`, because axum defines its own
//! `Message` and pins a tungstenite version of its own, and a
//! repository can easily hold two incompatible tungstenites at once —
//! this one does. A provider on some third transport keeps the frame
//! layer and leaves this module off.
//!
//! # What is here
//!
//! [`router`], the read loop: frames off the socket, forwarded to
//! whoever is waiting. It is the mirror of
//! [`client::router`](crate::client::router), and where it is not, the
//! reason is always the same one: a client opens scopes and a server
//! answers in them, so what one side registers in advance the other
//! learns from a frame.
//!
//! The write half is not here yet. When it is, it will be what
//! [`Sent`](router::Sent) is addressed to.
//!
//! There was a handler tree here before any of this, written before the
//! client half had a shape. It is gone rather than carried — what
//! answers requests should be built knowing how the routing was
//! solved, not around a sketch that predates it.

pub mod router;
