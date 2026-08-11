//! The wire frame.
//!
//! Every message is one WebSocket BINARY frame:
//!
//! ```text
//! [type: u8][scope: varint][channel: varint][payload…]
//! ```
//!
//! `type` leads because it is the only fixed-width field: a reader
//! knows what the frame is before parsing anything of variable length.
//! No length prefix — WebSocket already delimits messages, so carrying
//! one would be paying twice for the same fact.
//!
//! `scope` and `channel` are `u32`, varint-encoded, so a young
//! connection spends three bytes on a header and only pays for digits
//! it is actually using.
//!
//! # No WebSocket dependency
//!
//! Decoding takes `&[u8]` and encoding gives `Vec<u8>`, and nothing
//! here names a WebSocket type. There is no single type to name: axum
//! defines its own `Message` rather than re-exporting tungstenite's,
//! and pins a tungstenite version of its own besides — so naming
//! either would serve one library at one version and exclude the rest,
//! including the ones this repo already uses.
//!
//! Every `Message::Binary` carries `bytes::Bytes`, which derefs to
//! `&[u8]`. That is the one thing they all agree on, so it is what
//! this takes:
//!
//! ```ignore
//! Message::Binary(b) => ClientFrame::decode(&b)?
//! socket.send(Message::Binary(frame.encode().into())).await
//! ```
//!
//! # Scopes and channels
//!
//! A **scope** is one client request and everything that follows from
//! it. The client does not choose it: it sends a request with no
//! scope, and the server mints one in its ack. Every later frame in
//! either direction carries it.
//!
//! A **channel** is one exchange inside a scope. Channel `0` is the
//! answer to the client's own request. Any other channel is a request
//! the SERVER made, and only the server opens them — which is why the
//! two ends can never collide over one.
//!
//! # Types
//!
//! | type | meaning |
//! |------|---------|
//! | 0    | ack |
//! | 1    | body |
//! | 2    | finish |
//! | 3+   | a request |
//!
//! Ack, body and finish mean the same thing in both directions and on
//! every channel: an exchange beginning, its contents, and its end.
//! One sequence, whether the server is answering the client's request
//! on channel `0` or the client is answering a server request on that
//! request's channel.
//!
//! Everything from `3` up is a request. A client has exactly one —
//! the one that opens a scope — so it uses `3` and nothing else. A
//! server may have many kinds, and what each value means belongs to
//! the protocol being carried rather than to this layer.
//!
//! Because the reply types are shared and the request types are not,
//! `type` alone determines what a frame is. Nothing has to consult
//! whether the scope happens to be zero.

pub mod client;
pub mod server;

mod error;
mod varint;

pub use error::FrameError;
use error::{header_len, split_header, write_header};
