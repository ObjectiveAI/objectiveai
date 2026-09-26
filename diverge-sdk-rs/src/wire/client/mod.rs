//! The frame-level client: the half that mints scopes and channels.
//!
//! What a caller of the [`provider`](crate::provider) protocol holds,
//! and what a provider's server holds toward every container's proxy
//! — the [`outside`](crate::container_proxy::outside) wire it dials
//! into a container, where it mints the scopes and the proxy answers.
//! Same frame, same machinery, one implementation. The caller's own
//! answerers — what a provider asks a caller for while a container
//! runs — are the provider protocol's, in
//! [`provider::client`](crate::provider::client); nothing here knows
//! an endpoint.
//!
//! # What is here
//!
//! One socket: authenticated first, then split in two.
//!
//! [`authorize`] is the step in front. Whichever side dialled presents
//! the connection's one credential before anything else — this end's
//! going out, or the far end's being judged by the
//! [`unbrokered_authorizer`] this end supplies — and the
//! [`Connection`](crate::wire::connection::Connection) comes back out
//! ready to be split, as an [`authorized`] that names the far end
//! beside it. See [`authorization`] for which side does which, and
//! where the name comes from.
//!
//! [`router`] is the read loop: frames off the socket, forwarded to
//! whoever is waiting. [`handle`] is the write half and what a caller
//! keeps hold of. They are separate because a
//! [`Sink`](futures_util::Sink) needs `&mut` and a read loop never
//! finishes, so one type holding both could only ever do one of them.
//!
//! Two channels run between them. [`registration`] goes forward,
//! saying where frames should go before the request that causes them;
//! closures come back, saying an entry is gone.
//!
//! [`scope`] and [`channel`] are what a [`handle`] hands back — the
//! two things a caller opens, each with the receivers its frames
//! arrive on. Those three are data and nothing else, which is why
//! they sit beside the handle and the router rather than inside
//! either: what makes one is the writer's business, what routes by
//! one is the reader's, and what to do with one is a caller's.
//!
//! # Every queue here is unbounded
//!
//! So nothing this half does ever waits on a consumer. A [`router`]
//! forwarding a frame pushes it and moves on; one scope that has
//! stopped reading cannot stall another, and cannot stall the socket.
//!
//! Which is the same trade [`server`](crate::wire::server) makes, and
//! it is a trade. Dropping a frame was never on the table — a stream
//! has no way to say it lost one — so what a bound would have bought
//! is a stall, and what its absence costs is memory. An unread queue
//! grows at whatever rate the far end is sending, and nothing in this
//! crate bounds it.
//!
//! So the obligation on a caller is sharper than a depth would have
//! made it, not softer: read what you asked for, or drop it. Dropping
//! frees the queue and makes the router's sends fail, which it
//! ignores.

pub mod authorization;
pub mod authorize;
pub mod authorized;
pub mod channel;
pub mod handle;
pub mod registration;
pub mod router;
pub mod scope;
pub mod unbrokered_authorizer;
