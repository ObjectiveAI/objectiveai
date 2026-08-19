//! The caller half, behind the `client` feature.
//!
//! The mirror of [`server`](crate::server), and off for the same
//! reason: a provider should not have to compile a caller to answer a
//! frame.
//!
//! # What a caller supplies
//!
//! A handler, for the channels the far end opens: serving an image,
//! running a command, proxying Postgres. A caller is not only a source
//! of requests, so it is not only a client in the ordinary sense.
//!
//! # What is here
//!
//! One socket, split in two.
//!
//! [`router`] is the read loop: frames off the socket, forwarded to
//! whoever is waiting. [`handle`] is the write half and what a caller
//! keeps hold of. They are separate because a
//! [`Sink`](futures_util::Sink) needs `&mut` and a read loop never
//! finishes, so one type holding both could only ever do one of them.
//!
//! Two channels run between them, both unbounded. [`registration`]
//! goes forward, saying where frames should go before the request that
//! causes them; closures come back, saying an entry is gone.
//!
//! [`scope`] and [`channel`] are what a [`handle`] hands back — the
//! two things a caller opens, each with the receivers its frames
//! arrive on.
//!
//! Those three are data and nothing else, which is why they sit beside
//! the handle and the router rather than inside either: what makes one
//! is the writer's business, what routes by one is the reader's, and
//! what to do with one is a caller's.
//!
//! # Reading what comes back
//!
//! [`scope_response_stream`] and [`channel_response_stream`] are what a
//! caller DOES with the receivers those two hand over. Both are
//! [`Stream`](futures_util::Stream)s that take one frame off a queue,
//! strip the envelope, and hand the payload to a function the endpoint
//! supplies; [`response_stream_error`] is what either of them says when
//! it stops without ending.
//!
//! They are behaviour rather than data, and they still sit out here
//! rather than in [`handle`] or [`router`], because they belong to
//! neither half of the socket. Everything they touch has already
//! crossed it.
//!
//! Nothing about them is endpoint-specific — which is the point. The
//! only part of reading a stream that ever differs is turning one
//! payload into one item, and that is the one thing they take as an
//! argument.
//!
//! The socket itself is not here.
//! [`Connection`](crate::connection::Connection) carries either kind
//! under either half, because which end dialled is not a fact about
//! the protocol.

pub mod channel;
pub mod channel_response_stream;
pub mod handle;
pub mod registration;
mod response_stream;
pub mod response_stream_error;
pub mod router;
pub mod scope;
pub mod scope_response_stream;
