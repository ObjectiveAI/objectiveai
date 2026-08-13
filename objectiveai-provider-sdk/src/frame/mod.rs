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
//! Decoding takes `&[u8]`, and nothing here names a WebSocket type. There is no single type to name: axum
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
//! | type | client | server |
//! |------|--------|--------|
//! | 0    | auth | auth |
//! | 1    | — | response ack |
//! | 2    | — | response |
//! | 3    | — | response finish |
//! | 4    | channel response ack | channel response ack |
//! | 5    | channel response | channel response |
//! | 6    | channel response finish | channel response finish |
//! | 7    | agentic loop request | — |
//! | 8    | images check request | — |
//! | 9-127 | reserved | reserved |
//! | 128+ | a channel request | a channel request |
//!
//! Auth leads because it comes first in time: nothing may precede it,
//! and a peer reading a connection's opening byte should not have to
//! look past the reply types to find out whether it is one.
//!
//! `1` through `3` are the SCOPE-level replies — minting a scope,
//! answering on channel `0`, ending it — and only a server sends
//! those, which is why the client's are blank rather than reused.
//! `4` through `6` are the same three at CHANNEL level, and both sides
//! send them: each answers a channel the OTHER side opened.
//!
//! So `0` through `6` mean one thing apiece regardless of direction,
//! and only requests differ — which is the whole reason the client's
//! gap was left open instead of closed up.
//!
//! The three response frames — the ack, the response itself, and
//! the finish — mean the same thing in both directions and on every
//! channel: an exchange beginning, its contents, and its end.
//! One sequence, whether the server is answering the client's request
//! on channel `0` or the client is answering a server request on that
//! request's channel.
//!
//! # Requests
//!
//! Two kinds, and the split is by what they OPEN.
//!
//! `7` and `8` open a SCOPE, and only a client sends them — a server
//! never initiates one. They are a closed set this layer knows by
//! name: each is a variant of [`ClientFrame`] with its payload
//! decoded rather than carried, and a third would be `9`.
//!
//! `128` and above open a CHANNEL, and both sides
//! send them. That space is open: this layer knows only that a request
//! arrived, the payload stays bytes, the type stays a number, and an
//! unfamiliar one is a newer peer rather than an error.
//!
//! Which is why the boundary sits so far above what either side uses
//! today. Everything below it is this layer's own vocabulary and has
//! to be understood to be handled at all; everything above it is
//! somebody else's and can be passed along unread. Leaving `9` to
//! `127` empty costs nothing and means the two spaces never have to
//! be renumbered around each other.
//!
//! [`ClientFrame`]: client::ClientFrame
//!
//! # Auth
//!
//! A connection may be dialled from either end: a client to a server,
//! or a server to a client. Whichever side DIALLED sends an auth frame
//! first. Nothing may precede it.
//!
//! There is no answer. A credential that is accepted is followed by
//! the connection simply working; one that is not is followed by a
//! close, and by nothing else.
//!
//! That is deliberate, and it is the cheaper side of the trade. A peer
//! that has not authenticated cannot make the far end compose a reply,
//! so a bad credential costs its sender a socket and earns it nothing:
//! no bytes to amplify, and no answer to read a reason out of. Wrong,
//! expired and unknown are indistinguishable from outside, which is
//! the point — a rejection that explains itself is an oracle.
//!
//! What it costs is diagnosis. An honest client with a stale token
//! learns that its connection closed and not why, and has to work that
//! out from what it knows rather than from what it was told.
//!
//! Both directions carry the frame, because either may be the side
//! that dialled. Which end may send it is a fact about the connection
//! rather than about any frame, so the types do not express it.
//!
//! Auth frames have no scope and no channel. They come before either
//! exists, and are what makes it possible for one to.
//!
//! The payload is arbitrary. What counts as a credential is for the
//! two ends to agree — this layer guarantees only that it comes
//! first.
//!
//! Because the reply types are shared and the request types are not,
//! `type` alone determines what a frame is. Nothing has to consult
//! whether the scope happens to be zero.

pub mod client;
pub mod server;

mod error;

pub use error::{FrameError, HEADER_LEN};
use error::split_header;
