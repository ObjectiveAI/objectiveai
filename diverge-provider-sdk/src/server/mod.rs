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
//! # Empty
//!
//! There was a handler tree here — a connection at the root, a scope
//! under it, channels below that — and it was written before the shape
//! of the client half was. It is gone rather than carried: the client
//! now has a [`Router`](crate::client::router::Router) that reads
//! frames and forwards them, and whatever answers requests here has the
//! same problem to solve and should be built knowing how that one was
//! solved, not around a sketch that predates it.
//!
//! What went with it was one real path, an unreadable request answered
//! with an error and a finish, and a good deal of scaffolding that had
//! `unimplemented!` where every decision belonged.
//!
//! Nothing outside this module noticed. A message is the same message
//! whether or not somebody compiled the code that answers it, which is
//! what the split between this and [`frame`](crate::frame) is for.
