//! The provider half, behind the `server` feature.
//!
//! Everything else in this crate is a message. This is the part that
//! does something with one, and it is opt-in for exactly that reason:
//! a client, and a tool that only inspects the protocol, should not
//! have to compile a provider to read a frame.
//!
//! # The socket, and only the socket
//!
//! The feature brings `axum` with `default-features = false` and `ws`
//! alone: the WebSocket handshake and
//! `axum::extract::ws::{WebSocket, Message}`, and neither `http1` nor
//! `http2`.
//!
//! So nothing in this crate can accept a connection, route a request,
//! or answer one. A provider stands up its own server, upgrades
//! whatever path it likes, and hands over the socket that comes out.
//!
//! Which keeps the choice where it belongs. Where the endpoint lives,
//! what authenticates the upgrade, what else that server does — none
//! of it is this protocol's business, and a crate that served HTTP
//! would have opinions about all three.
//!
//! # Nothing is re-exported
//!
//! A provider that hands over a socket already depends on axum, and
//! naming `axum::extract::ws::WebSocket` through this crate would put
//! a second path on somebody else's type — one that says nothing new
//! and goes stale the day the version underneath it moves. What this
//! module takes, it takes by that name.
//!
//! # Why axum's types rather than tungstenite's
//!
//! Because [`frame`](crate::frame) deliberately names neither. It
//! decodes `&[u8]`, since axum defines its own `Message` and pins a
//! tungstenite version of its own, and a repository can easily hold
//! two incompatible tungstenites at once — this one does.
//!
//! So the frame layer stays free of both, and the choice happens here,
//! behind a flag. A provider that wants a different transport keeps
//! the frame layer and leaves this module off.
//!
//! # Empty
//!
//! Nothing here yet. It lands incrementally, and nothing outside this
//! module changes as it does — a message is the same message whether
//! or not somebody compiled the code that answers it.
