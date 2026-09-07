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
//! of requests, so it is not only a client in the ordinary sense —
//! though today only the plumbing is here, and the handlers come with
//! the executors.
//!
//! # What is here
//!
//! One socket: authenticated first, then split in two.
//!
//! [`authorize`] is the step in front. Whichever side dialled presents
//! the connection's one credential before anything else — this end's
//! going out, or the provider's being judged by the
//! [`unbrokered_authorizer`] a caller supplies — and the
//! [`Connection`](crate::connection::Connection) comes back out ready
//! to be split. See [`authorization`] for which side does which.
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
//! arrive on.
//!
//! # Every queue here is unbounded
//!
//! So nothing this half does ever waits on a consumer. A [`router`]
//! forwarding a frame pushes it and moves on; one scope that has
//! stopped reading cannot stall another, and cannot stall the socket.
//!
//! Which is the same trade [`server`](crate::server) makes, and it is a
//! trade. Dropping a frame was never on the table — a stream has no way
//! to say it lost one — so what a bound would have bought is a stall,
//! and what its absence costs is memory. An unread queue grows at
//! whatever rate the far end is sending, and nothing in this crate
//! bounds it.
//!
//! So the obligation on a caller is sharper than a depth would have
//! made it, not softer: read what you asked for, or drop it. Dropping
//! frees the queue and makes the router's sends fail, which it ignores.
//!
//! Those three are data and nothing else, which is why they sit beside
//! the handle and the router rather than inside either: what makes one
//! is the writer's business, what routes by one is the reader's, and
//! what to do with one is a caller's.
//!
//! # Answering what the far end asks
//!
//! A caller is not only a source of requests. A provider opens channels
//! back into it for the things it cannot reach itself — an MCP server,
//! a database, a registry, a command, a vault, mounted content — and
//! something has to answer them. The traits that do are not here yet:
//! they went with the endpoints they served, and the ones the
//! [`containers`](crate::endpoints::containers) family needs land with
//! its executors. What is here is the plumbing every executor rides.
//!
//! The socket itself is not here.
//! [`Connection`](crate::connection::Connection) carries either kind
//! under either half, because which end dialled is not a fact about
//! the protocol.

pub mod authorization;
pub mod authorize;
pub mod channel;
pub mod handle;
pub mod registration;
pub mod router;
pub mod scope;
pub mod unbrokered_authorizer;
