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
//! a database, a registry, a command — and something has to answer
//! them.
//!
//! There are six, one per thing that can be asked — the
//! [`unbrokered_authorizer`] above is a seventh supplied trait, but it
//! answers the connection rather than anything asked on it.
//! [`mcp_proxy`] forwards an exchange to a server the provider cannot
//! see. [`fetch_proxy`] hands over a mounted file, a mounted
//! directory or a resource the provider is missing, out of the
//! caller's own store, by its size-bearing identity — and the
//! continuation a run resumes from, which needs no name.
//! [`oci_proxy`] serves an image, for a plugin run and a
//! laboratory run alike. [`command_proxy`] runs a command a plugin has
//! no binary for. [`postgres_proxy`] splices a connection onto the
//! caller's database, and is handed the request that started the
//! plugin so that a caller can decide what that connection may reach.
//!
//! Which is one of the things here that name a type from
//! [`endpoints`](crate::endpoints). All do it for the same reason:
//! what they have to decide is that endpoint's question, and no
//! [`shared`](crate::shared) type says it. See each trait for the
//! whole of that argument.
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
//! the CLI's own shape, the empty finish. None of them has an error
//! variant, because a second way to say a thing is a second thing to
//! disagree about.
//!
//! The roster is finished, and the sixth is not a proxy.
//! [`laboratory_connection_authorizer`] decides whether a connector may
//! join a running laboratory, and there is nothing on the other side of
//! it to forward to — it is asked a question and it answers.
//!
//! Which makes it the one whose answer is not an aside. The five above
//! have no error variant because a refusal already has somewhere to
//! live; here a refusal IS the answer, and a denial is as ordinary an
//! outcome as an admission.
//!
//! The other thing a laboratory run asks for is the content of a write,
//! and that needed no trait at all: content is an argument to the write
//! a caller already asked for, not a service a caller provides.
//!
//! # Who asks for them
//!
//! [`agentic_loop::run`](crate::endpoints::agentic_loop::run::client::execute)
//! takes an [`mcp_proxy`] and a [`fetch_proxy`];
//! [`mcp_plugin::run`](crate::endpoints::mcp_plugin::run::client::execute)
//! takes an [`oci_proxy`], a [`postgres_proxy`] and a
//! [`command_proxy`];
//! [`laboratories::run`](crate::endpoints::laboratories::run::client::execute)
//! takes an [`oci_proxy`] and a
//! [`laboratory_connection_authorizer`].
//!
//! [`laboratories::connect`](crate::endpoints::laboratories::connect::client::execute)
//! takes none, because a connector is asked for one thing and that
//! thing is an argument rather than a service.
//!
//! # Two things true of all six
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

pub mod authorization;
pub mod authorize;
pub mod channel;
pub mod command_proxy;
pub mod fetch_proxy;
pub mod handle;
pub mod laboratory_connection_authorizer;
pub mod mcp_proxy;
pub mod oci_proxy;
pub mod postgres_proxy;
pub mod registration;
pub mod router;
pub mod scope;
pub mod unbrokered_authorizer;
