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
//! a database, a registry, a command — and something has to answer
//! them.
//!
//! There are four so far, one per thing that can be asked.
//! [`mcp_proxy`] forwards an exchange to a server the provider cannot
//! see. [`oci_proxy`] serves an image, for a plugin run and a
//! laboratory run alike. [`command_proxy`] runs a command a plugin has
//! no binary for. [`postgres_proxy`] splices a connection onto the
//! caller's database.
//!
//! They are traits rather than callbacks because each has its own
//! shape, and the shapes really are different: one request one answer,
//! one request many answers, and — for a database — a PAIR of channels,
//! one per direction, because only a responder can end a channel and a
//! connection has to be endable from both sides. Only [`oci_proxy`]
//! reads like [`mcp_proxy`], and only because both are tunneled HTTP.
//!
//! What they share is one idea rather than one signature — each punts
//! failure into a vocabulary that already exists, and each punts to a
//! different one. An HTTP status, a pgwire `ErrorResponse`, an item in
//! the CLI's own shape. None of them has an error variant, because a
//! second way to say a thing is a second thing to disagree about.
//!
//! The roster is not finished. A [`laboratory run`](crate::endpoints::laboratories::run::server::channel_request::Frame)
//! also asks for an authorization and for content to write, and those
//! need two more — the authorization being the one case whose answer is
//! genuinely shaped like a [`Result`], since its frame has variants to
//! say so.
//!
//! None of them is wired to anything yet. What reads a scope's channel
//! requests and dispatches to one is not written, and Postgres is the
//! one that will not fall out of a task per frame: a connection request
//! has to open a channel BACK before it can be served, and the writes
//! then arrive as that channel's responses rather than as further
//! requests.
//!
//! # Two things true of all four
//!
//! None is `dyn`-compatible, because each returns `impl Future`. So a
//! dispatcher is generic over the ones it needs at once rather than
//! holding boxes — which monomorphizes free and costs nothing, unless a
//! caller wants to choose a proxy at runtime, whose answer is an enum
//! of their own with one impl on it.
//!
//! None can exert backpressure, for the reason the section above gives.
//! A command yielding a million items queues them into the sink as fast
//! as the socket takes them, and every one of those frames takes the
//! handle's lock in turn against every other write on the connection.
//! The lock is held for one frame and never for a stream of them, so
//! nothing is starved — but a caller writing a firehose should know it
//! is contending.
//!
//! The socket itself is not here.
//! [`Connection`](crate::connection::Connection) carries either kind
//! under either half, because which end dialled is not a fact about
//! the protocol.

pub mod channel;
pub mod command_proxy;
pub mod handle;
pub mod mcp_proxy;
pub mod oci_proxy;
pub mod postgres_proxy;
pub mod registration;
pub mod router;
pub mod scope;
