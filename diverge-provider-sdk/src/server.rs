//! The provider half, behind the `server` feature.
//!
//! Everything else in this crate is a message. This is the part that
//! does something with one, and it is opt-in for exactly that reason:
//! a client, and a tool that only inspects the protocol, should not
//! have to compile a provider to read a frame.
//!
//! # The socket, and only the socket
//!
//! [`WebSocket`] and [`Message`] are axum's, re-exported so a provider
//! and this crate name the same types rather than one converting into
//! the other's.
//!
//! What is NOT here is the serving. The dependency is `axum` with
//! `default-features = false` and `ws` alone — no `http1`, no `http2`,
//! no extractors — so nothing in this crate can accept a connection,
//! route a request, or answer one. A provider stands up its own
//! server, upgrades whatever path it likes, and hands over the socket
//! that comes out.
//!
//! Which keeps the choice where it belongs. Where the endpoint lives,
//! what authenticates the upgrade, what else that server does — none
//! of it is this protocol's business, and a crate that served HTTP
//! would have opinions about all three.
//!
//! # Why axum's types rather than tungstenite's
//!
//! Because [`frame`](crate::frame) deliberately names neither. It
//! decodes `&[u8]`, since axum defines its own `Message` and pins a
//! tungstenite version of its own, and a repository can easily hold
//! two incompatible tungstenites at once — this one does.
//!
//! So the frame layer stays free of both, and the naming happens here,
//! once, behind a flag. A provider that wants a different transport
//! keeps the frame layer and leaves this module off.
//!
//! # Empty otherwise
//!
//! Nothing else yet. It lands incrementally, and nothing outside this
//! module changes as it does — a message is the same message whether
//! or not somebody compiled the code that answers it.

pub use axum::extract::ws::{Message, WebSocket};
