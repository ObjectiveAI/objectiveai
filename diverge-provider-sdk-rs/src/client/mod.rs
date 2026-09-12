//! The caller half, behind the `client` feature.
//!
//! The mirror of [`server`](crate::server), and off for the same
//! reason: a provider should not have to compile a caller to answer a
//! frame.
//!
//! # What a caller supplies
//!
//! Requests, and answers. A caller opens scopes and channels — every
//! endpoint's `client::execute` performs one exchange rather than
//! describing it — and a provider opens channels back into it for
//! the things it cannot reach itself, which the caller answers
//! through the traits below. Every endpoint has its executor: the
//! five `volumes`, `images::check` and `version` collapse into a call
//! or a stream; the three `containers` scopes hand back a handle that
//! holds the container's life. The client half is complete.
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
//! While a container runs, its provider opens channels into the
//! caller for what lives with the caller, and a caller supplies one
//! answerer per kind, assembled in an [`Answerers`]:
//!
//! | trait | answers |
//! |-------|---------|
//! | [`OciStore`] | the manifest and blobs of an image the caller holds |
//! | [`ConnectionAuthorizer`] | whether a connector may attach |
//! | [`IdentityStore`] | mounted content the provider does not hold |
//! | [`PostgresDialer`] | the container's database connections |
//! | [`CommandRunner`] | the commands the container asks run |
//! | [`Vault`] | the container's secrets, with locks |
//! | [`McpServer`] | the container's tool calls outward |
//! | [`FuseServer`] | the files and directories mounted live |
//!
//! Each answers in the wire's own vocabulary — an absence is the
//! empty finish, a refusal the frame that says so — so none carries
//! an error type of its own. The executors under
//! [`containers`](crate::endpoints::containers) read every ask off
//! the scope, decode it, and answer it on a task of its own through
//! these; the connect scope, which is asked for nothing but the
//! content of its own writes, takes none of them.
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

mod answerers;
mod command_runner;
mod connection_authorizer;
mod fuse_server;
mod identity_store;
mod mcp_server;
mod oci_store;
mod postgres_dialer;
mod vault;

pub use answerers::*;
pub use command_runner::*;
pub use connection_authorizer::*;
pub use fuse_server::*;
pub use identity_store::*;
pub use mcp_server::*;
pub use oci_store::*;
pub use postgres_dialer::*;
pub use vault::*;
