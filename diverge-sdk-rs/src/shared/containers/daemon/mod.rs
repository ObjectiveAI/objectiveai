//! A daemon connection, carried frame by frame.
//!
//! A container has no daemon it may dial, and the program in it is a
//! client of the daemon all the same: it speaks the
//! [daemon protocol](crate::daemon) to its proxy, on the loopback's
//! `/daemon`, exactly as a client speaks it to the daemon — scopes,
//! channels, every endpoint — and the proxy carries each frame to the
//! provider, and the provider to the caller, which is the daemon. The
//! ask is one [`request::Request`], one client frame of that
//! connection; the answer is a stream of [`response::Frame`]s, the
//! server frames that answer it, then the finish — or an
//! [`Error`](response::Frame::Error) last, when the caller could not
//! serve the frame at all.
//!
//! # One channel per client frame
//!
//! The wire inside is the wire outside, nested: a scope the program
//! opens is a scope of the daemon's, and the daemon's channels on it
//! come back to the program as they would to any client. Each frame
//! the program sends is exactly one channel of this kind, and what
//! that channel answers with is settled by the frame it carries:
//!
//! | the program sends | the channel answers with | and finishes when |
//! |-------------------|--------------------------|-------------------|
//! | `Request { scope, … }` | every server frame of that scope — its responses, the daemon's channel requests on it, their finishes — in order | the scope's `ResponseFinish` has been sent |
//! | `ChannelRequest { scope, channel, … }` | every `ChannelResponse` and the `ChannelResponseFinish` of that channel | the channel's finish has been sent |
//! | `ChannelResponse` or `ChannelResponseFinish` | nothing | at once |
//!
//! The nested scope and channel numbers ride inside the frames, which
//! carry them already, so nothing between the program and the daemon
//! routes: the proxy writes every server frame it is answered with to
//! the program's socket as it is. The caller keeps one daemon session
//! per container and feeds every frame into it.
//!
//! # No credential passes
//!
//! A daemon connection inside a container is served for the
//! container's [`account`](crate::daemon::create::Inner::account): the
//! proxy is the trust boundary, and an `Auth` frame is the one client
//! frame this ask cannot carry — it does not decode, and the proxy
//! closes `/daemon` on one. The server frame of the same name is left
//! out the same way.
//!
//! # The frame is the whole message
//!
//! A frame is not read by anything it passes through: the proxy, the
//! provider and the caller's relay forward its bytes, and only the
//! daemon's session reads the request inside. An [`Error`] answers a
//! frame the caller could not serve — the session gone, a frame on a
//! scope it does not have — and the proxy turns it into the wire's own
//! word for "not served": a `ResponseFinish` for a `Request`, a
//! `ChannelResponseFinish` for a `ChannelRequest`, nothing for the
//! rest.
//!
//! The same shapes ride both wires this crate defines: the provider's
//! channel toward the caller, and the
//! [`proxy`](crate::container_proxy::outside::endpoints::agents::begin)
//! inside the container.
//!
//! [`Error`]: response::Frame::Error

pub mod request;
pub mod response;
