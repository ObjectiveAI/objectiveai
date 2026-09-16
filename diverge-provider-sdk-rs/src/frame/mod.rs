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
//! it. The client chooses it and puts it in the request's header, and
//! every later frame in either direction carries it. Nothing else
//! opens a scope, so there is one minter and nothing to collide
//! with.
//!
//! A **channel** is one exchange inside a scope. Channel `0` is the
//! answer to the client's own request. Any other channel was opened by
//! a channel request, and BOTH sides open them.
//!
//! Which is why the numbering is per SENDER. A client's channel `5`
//! and a server's channel `5` are different channels, told apart by
//! the direction a frame travelled rather than by the number — so
//! neither end has to know what the other has minted, and neither can
//! collide with it.
//!
//! # One request per channel
//!
//! A channel carries exactly ONE request. There is never a second
//! request frame on a live channel, and the number may not be used
//! again until the response stream on it has finished.
//!
//! Both halves of that are the same rule seen from either end, and it
//! holds for both sides — a server opening two requests on one of its
//! channels breaks it exactly as a client would.
//!
//! It is what makes a channel mean something. A channel that took
//! several requests would have no answer to when its number came free,
//! because the responses to the first and the third would be
//! indistinguishable; and it would have no way to END, because
//! finishing is a responder's act and a requestor has no frame with
//! which to say it has stopped asking.
//!
//! # So a duplex exchange is TWO channels
//!
//! Which is the shape everything bidirectional in this protocol takes,
//! rather than a limitation any of them work around. A write names its
//! destination on the client's channel and streams its content back on
//! one the provider opens; an image pull does the same; a Postgres
//! connection is one channel per direction, correlated by an id that
//! belongs to the connection rather than to either channel.
//!
//! It costs a round trip before the first byte and buys two channels
//! that each do one thing — and, more than that, it buys a close in
//! both directions. Each side finishes the channel it is answering on,
//! so each side can say it is done, which a single duplex channel
//! could never have expressed.
//!
//! # A stream ends at its finish frame, and nowhere else
//!
//! This holds everywhere, for every stream in this protocol. A quiet
//! channel is a channel still running, however long it has been quiet.
//! Nothing times one out, and a reader waits.
//!
//! So a pause is never an ending, and a sender that stops without
//! finishing has not cancelled anything — it has left a stream open.
//! A sender with something to say says it in a frame.
//!
//! # Types
//!
//! | type | client | server |
//! |------|--------|--------|
//! | 0    | auth | auth |
//! | 1    | request | — |
//! | 2    | — | response |
//! | 3    | — | response finish |
//! | 4    | channel request | channel request |
//! | 5    | channel response | channel response |
//! | 6    | channel response finish | channel response finish |
//!
//! Auth leads because it leads in time: nothing may precede it, and a
//! peer reading a connection's opening byte should not have to look
//! past the reply types to find out whether it is one.
//!
//! After it the order is the order things happen in. A request opens
//! something — a scope at `1`, a channel at `4` — and the responses
//! and finish that answer it follow immediately behind. The same three
//! beats twice, once per level.
//!
//! A number means one thing in both directions, and the blanks are
//! what keep it that way. Only a client opens a scope, so only a
//! client sends `1`; only a server answers one, so only a server sends
//! `2` and `3`. Closing those gaps would save two byte values and cost
//! the property that a type identifies a frame without reference to
//! who sent it.
//!
//! # Nothing is acknowledged
//!
//! An opening used to be answered before it was answered: a scope got
//! an ack, and so did a channel.
//!
//! The scope ack had a job — it was where the scope came from — and
//! the requestor choosing its own took that away. The channel acks
//! never had one: a channel has always been numbered by whoever opened
//! it, so those two only ever said "received".
//!
//! Which is not worth a frame. A peer too busy to answer is a peer too
//! busy to acknowledge, so the signal is thinnest exactly when someone
//! would want it, and it adds to what is already too much. A sender
//! learns its request landed by being answered; a sender that wants a
//! deadline keeps one itself, because nothing here will.
//!
//! # Requests
//!
//! Two kinds, and the split is by what they OPEN. `1` opens a SCOPE
//! and only a client sends it; `4` opens a CHANNEL and both sides
//! send it.
//!
//! Neither is discriminated here. A scope request's tag chooses among
//! the twelve in [`ClientRequest`](crate::endpoints::ClientRequest), and a
//! channel request's means something only inside the scope it arrived
//! in — but both are read by whoever is going to act on them, and this
//! layer hands over bytes either way.
//!
//! A malformed scope request is still answerable, and for the same
//! reason it always was: decoding one is
//! [`Infallible`](std::convert::Infallible), so a payload nobody can
//! name becomes
//! [`Invalid`](crate::endpoints::ClientRequest::Invalid) rather than a
//! failure. What changed is only WHERE that happens — a server decodes
//! when it goes to answer, in the scope the client opened, instead of
//! this layer deciding on its behalf.
//!
//! # Growth happens in payloads, not here
//!
//! Every type is fixed and enumerated, which is why an unfamiliar one
//! is malformed rather than tolerated. A protocol that grows a new
//! kind of channel request grows a tag value inside a payload, where
//! the reader that cares is already looking — this layer never has to
//! learn it, and never has to hold room open for it.
//!
//! A new SCOPE request is the exception, and it is an exception in
//! [`endpoints`](crate::endpoints) rather than here: it takes the next
//! tag value and a variant beside the others. The frame types are
//! untouched either way.
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
//! An auth frame carries bytes, like every other frame here. What they
//! mean is [`Auth`](auth::Auth), and reading them is a second step the
//! holder takes — there is nobody else to hand them to, since a
//! credential belongs to the CONNECTION rather than to any scope, but
//! that is a reason to decode them promptly and not a reason for the
//! envelope to insist.
//!
//! What is INSIDE the credential is nobody's business here at all.
//! [`Auth`](auth::Auth) says which mode it arrived in and hands back a
//! string; what counts as an acceptable one is for the two ends to
//! agree, and there is a second mode coming in which a broker is the
//! one that agrees.
//!
//! Where a type exists in both directions it means the same thing in
//! both, so `type` alone determines what a frame is. Nothing has to
//! consult whether the scope happens to be zero.

pub mod auth;
pub mod client;
pub mod server;

mod error;

pub use error::{FrameError, HEADER_LEN};
use error::split_header;
