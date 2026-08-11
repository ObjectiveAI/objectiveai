//! The wire frame.
//!
//! Every message is one WebSocket BINARY frame:
//!
//! ```text
//! [type: u8][scope: u32 big-endian][channel: u32 big-endian][payload…]
//! ```
//!
//! `type` leads so a reader learns what a frame is before anything
//! else, and can reject one it does not recognize without looking at
//! the rest. No length prefix — WebSocket already delimits messages,
//! so carrying one would be paying twice for the same fact.
//!
//! Nine bytes, always. Fixed rather than varint-encoded: varints
//! would have saved about four bytes on a typical frame, which is
//! under two percent of a chunk carrying JSON, and charged for it with
//! a payload offset that is a parse result instead of a constant and
//! an overflow case to get right. A header whose length is
//! [`HEADER_LEN`] is a header nobody has to parse.
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

pub use error::{FrameError, HEADER_LEN};
use error::{split_header, write_header};
