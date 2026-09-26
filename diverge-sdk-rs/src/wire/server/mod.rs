//! The frame-level server: the half that is told about scopes and
//! answers inside them.
//!
//! What a provider holds for every connection a caller makes, and
//! what the proxy inside a container holds for the provider's
//! connection into it. It takes a
//! [`Connection`](crate::wire::connection::Connection) — incoming or
//! outgoing, because a provider usually waits to be connected to and
//! sometimes connects to a caller it cannot otherwise reach, and
//! answers the same frames either way — and yields the scopes the far
//! end opens, each with the bytes that opened it and nothing decoded:
//! which request vocabulary those bytes are in is the
//! [`provider`](crate::provider::server::handle)'s or the proxy's to
//! know, not this half's.
//!
//! # The socket, and only the socket
//!
//! What this half cannot do is make one. The crate brings `axum`
//! without `http1` or `http2`, and `tokio-tungstenite` with `stream`
//! and `connect` — enough to name both socket types, and to dial the
//! one place this crate dials, not to serve. A provider stands up its
//! own server, or connects with its own client, and hands over what
//! comes out. Which keeps the choice where it belongs: where the
//! endpoint lives, what the URL is, what authenticates the upgrade,
//! what else that server or process does — none of it is a
//! protocol's business.
//!
//! # Nothing is re-exported
//!
//! A provider that hands over a socket already depends on axum, and
//! naming `axum::extract::ws::WebSocket` through this crate would put
//! a second path on somebody else's type — one that says nothing new
//! and goes stale the day the version underneath it moves. What
//! [`connection`](crate::wire::connection) takes, it takes by that
//! name. The [`frame`](crate::wire::frame) layer stays free of both
//! libraries regardless: it decodes `&[u8]`.
//!
//! # What is here
//!
//! Two types that do something, and the whole shape is in how they
//! divide.
//!
//! [`session`] is a connection: it takes one, splits it, and answers
//! with the scopes a client opens on it, as [`received`] things. A
//! [`scope_handle`] is one of those scopes: what was asked, the
//! channels the client opens inside it, and the channels this end
//! opens back.
//!
//! The reading is all in the first and the writing is all in the
//! second. So a [`Session`](session::Session) holds the read half and
//! shares the write half out, and a
//! [`ScopeHandle`](scope_handle::ScopeHandle) carries a share of it
//! along with its own inbox.
//!
//! [`channel`] is what a scope gets back when it opens one — a number
//! and the receiver its answers arrive on. Data and nothing else, which
//! is why it sits beside the handle rather than inside it: what makes
//! one is the handle's business, and what to do with one is a caller's.
//!
//! `notice` is the queue running the other way, from the scopes back to
//! the session, carrying the two things a session cannot work out for
//! itself: where a channel's answer should land, and what has ended. It
//! is private, because a session makes both ends of it and there is
//! nothing for a caller to wire up.
//!
//! # Why it is not the caller half turned around
//!
//! [`client`](crate::wire::client) is a router and a handle, and
//! neither shape survives the crossing. The reason is the one
//! asymmetry in the whole wire: **a client mints scope numbers and a
//! server only ever learns them.**
//!
//! A client arranges where a scope's frames will go before it opens
//! one, so something must hold those arrangements and match arriving
//! frames against them. That is routing, and it is a job worth a type.
//! Nothing here can arrange a SCOPE — a server is told about those —
//! so what would have been a router is a loop that reads frames and
//! hands out the ones that open something, which is what a
//! [`Session`](session::Session) is.
//!
//! Channels are the other way round: a channel this end opens is this
//! end's to number, and its answer needs somewhere to land before it
//! arrives. So a session routes for the half it opens and merely
//! delivers for the half it is told about, and the one queue running
//! back to it is what carries the difference.
//!
//! The handle divides the other way for the same reason. A client's is
//! a shared, cloneable writer that hands out scopes it CREATED, and
//! every clone can create another. A server creates none, so there is
//! nothing to share at that level: the writing belongs to the
//! individual scope, and that is where the connection's write half
//! ends up. Sharing happens one level down instead: a
//! [`ScopeHandle`](scope_handle::ScopeHandle) takes `&self`
//! throughout, so several tasks can answer one scope at once. A
//! client shares the thing that makes scopes; a server shares the
//! scope.
//!
//! # Auth is a handshake in front
//!
//! Whichever side dialled authenticates. On an
//! [`Incoming`](crate::wire::connection::Connection::Incoming)
//! connection the peer's first frame must be its credential, judged
//! by the [`unbrokered_authorizer`] this end supplies — the identity
//! it produces is what everything downstream receives. On an
//! [`Outgoing`](crate::wire::connection::Connection::Outgoing)
//! connection this end sends the credential before reading anything,
//! and the identity was never in question: this end dialled the peer,
//! so it already knows who it is. [`authorization`] is the argument
//! that says which. The provider's
//! [`handle`](crate::provider::server::handle) drives the handshake
//! in front of its dispatch, and its error type is where a handshake
//! that failed is reported — never to the peer, which is the auth
//! frame's own no-answer rule. The client half tells the same story
//! from the other chair: [`client::authorize`](crate::wire::client::authorize)
//! presents this end's credential or judges the far end's.

pub(crate) mod answer;
pub(crate) mod answers;
pub mod authorization;
pub mod channel;
mod notice;
pub mod received;
pub mod scope_handle;
pub mod session;
pub mod unbrokered_authorizer;
