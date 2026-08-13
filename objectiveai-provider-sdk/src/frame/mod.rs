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
//! | 1    | request | — |
//! | 2    | — | response ack |
//! | 3    | — | response |
//! | 4    | — | response finish |
//! | 5    | channel request | channel request |
//! | 6    | channel response ack | channel response ack |
//! | 7    | channel response | channel response |
//! | 8    | channel response finish | channel response finish |
//!
//! Auth leads because it leads in time: nothing may precede it, and a
//! peer reading a connection's opening byte should not have to look
//! past the reply types to find out whether it is one.
//!
//! After it the order is the order things happen in. A request opens
//! something — a scope at `1`, a channel at `5` — and the ack,
//! responses and finish that answer it follow immediately behind. The
//! same four beats twice, once per level.
//!
//! A number means one thing in both directions, and the blanks are
//! what keep it that way. Only a client opens a scope, so only a
//! client sends `1`; only a server answers one, so only a server sends
//! `2` through `4`. Closing those gaps would save two byte values and
//! cost the property that a type identifies a frame without reference
//! to who sent it.
//!
//! # Requests
//!
//! Two kinds, and the split is by what they OPEN. `1` opens a SCOPE
//! and only a client sends it; `5` opens a CHANNEL and both sides
//! send it.
//!
//! Neither is discriminated here. WHICH request either one is lives in
//! the payload's own leading byte, so this layer sees two frame kinds
//! however many kinds of request there turn out to be.
//!
//! # Growth happens in payloads, not here
//!
//! Every type is fixed and enumerated, which is why an unfamiliar one
//! is malformed rather than tolerated. A protocol that grows a new
//! kind of channel request grows a tag value inside a payload, where
//! the reader that cares is already looking — this layer never has to
//! learn it, and never has to hold room open for it.
//!
//! The alternative was a range of request types with the kind encoded
//! in `type`. It costs a second discriminator: a frame already carries
//! one payload's worth of protocol, so putting part of that protocol
//! in the envelope means two places to keep in step and two
//! vocabularies to version.
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
