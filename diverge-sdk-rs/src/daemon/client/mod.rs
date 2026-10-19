//! The caller half of the daemon protocol: performing every exchange.
//!
//! Every endpoint under [`endpoints`](crate::daemon::endpoints) has a
//! `client::execute` that performs its exchange rather than describing
//! it — encodes the request, opens a scope on a
//! [`Handle`](crate::wire::client::handle::Handle), reads what comes
//! back out of the frame envelope, and hands back the daemon's answer
//! typed, or an error naming which layer of the machinery failed — as
//! every endpoint of the [`provider`](crate::provider) has one. The
//! daemon's exchanges take four shapes, and each is written once here;
//! an endpoint's executor names its own types and calls the one it is.
//!
//! | shape | here | the endpoint's `execute` hands back |
//! |-------|------|-------------------------------------|
//! | one answer | [`one_shot`] | the response frame, whichever answer it is |
//! | a stream | [`stream`] | an [`ExecuteStream`] of response frames, ended by the finish |
//! | a stream with a cancel | [`stream`] and [`cancel`] | the stream, and a [`Cancel`] that ends it |
//! | one answer with a cancel | [`pending`] | a [`Pending`] that waits, or cancels |
//! | an upload | [`upload`] | the response frame, once every content channel the daemon opened is answered |
//!
//! # The answer is the frame
//!
//! A daemon response is one of several ANSWERS and one failure, and an
//! executor returns the whole frame: `Forbidden`, `NotFound`, `InUse`
//! and the daemon's own `Error` are all values a caller matches, since
//! each is something the daemon said. An executor's error is the
//! absence of an answer — the request that would not encode, the
//! connection gone, a frame this end cannot place, a response that
//! would not parse — and nothing the daemon meant.
//!
//! # Holding a handle
//!
//! A caller holds one [`Handle`](crate::wire::client::handle::Handle)
//! per connection, as a caller of a provider does: the connection is
//! [`authorize`](crate::wire::client::authorize)d first, this end
//! dialling and presenting the account's key — the one its
//! [`create`](crate::daemon::endpoints::accounts::create) answered — as
//! [`Authorization::Outgoing`](crate::wire::client::authorization::Authorization::Outgoing),
//! and then split into a
//! [`Router`](crate::wire::client::router::Router) that reads and the
//! handle that writes. A program in a container holds its handle from
//! its proxy instead —
//! [`Client::daemon`](crate::container_proxy::inside::Client::daemon)
//! dials the loopback's `/daemon`, which the proxy serves as the daemon
//! protocol, presenting nothing — and is served for the container's
//! account. Every executor takes the handle by reference and opens its
//! own scope on it; any number may run at once.

pub mod cancel;
pub mod one_shot;
pub mod pending;
pub mod stream;
pub mod upload;

pub use cancel::*;
pub use pending::Pending;
pub use stream::{Decoder, ExecuteStream, OpenError, StreamError};
pub use upload::{AskDecoder, UploadError};
