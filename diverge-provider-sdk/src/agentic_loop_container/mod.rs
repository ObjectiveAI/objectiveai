//! The HTTP surface every agentic-loop container serves.
//!
//! One definition, whatever the upstream behind it: a container
//! serves a WebSocket at `/` on the loop port — `8080`, per the
//! Container section of the provider specification. The server's
//! first binary message is THE request ([`request::Request`]), and
//! every message the container sends back is one
//! [`response::Response`] frame: the loop's chunks; the container's
//! own asks, which the server consumes and answers by POSTing bytes
//! back in — a [`fetch_resource`](response::FetchResource) ask
//! answered at `POST /resource/{identity}` ([`resource`]), and the
//! one [`fetch_continuation`](response::FetchContinuation) ask every
//! run opens with, answered at `POST /continuation`
//! ([`continuation`]), both chunked, both settled by the completion
//! or the error, a continuation delivered as a lone completion being
//! the fresh start; and, last, the run's new continuation as raw
//! bytes, the closer, after which the socket closes.
//! Beside it, the running conversation's queue has exactly two
//! verbs: [`enqueue`] puts a message in at `POST /enqueue`,
//! [`dequeue`] clears whatever has not yet been taken at
//! `POST /dequeue`. Every container speaks all of it identically,
//! which is what lets a provider relay a caller's
//! [`Enqueue`](crate::endpoints::agentic_loop::run::client::channel_request::Frame::Enqueue)
//! or
//! [`Dequeue`](crate::endpoints::agentic_loop::run::client::channel_request::Frame::Dequeue)
//! without knowing which container is behind it.
//!
//! # The database is a port, not a frame
//!
//! The caller's database is `127.0.0.1:14980` inside the container.
//! Nothing here models it, because nothing needs to — a container
//! that opens a TCP connection there speaks pgwire straight to
//! whatever the caller routes it to, one connection one session, and
//! the bytes are relayed unread in both directions: the container's
//! Postgres proxy accepts the connection and carries it to the server
//! over its own WebSocket on `14981`, per the
//! [`postgres_proxy`](crate::postgres_proxy) wire, and the server
//! carries it to the caller as the loop's channel pair. No credential
//! of the container's own is involved: the caller's proxy is the
//! authority on what the connection may reach. An upstream whose
//! state is rows points its driver at that address; one whose state
//! is files never dials it, and costs nothing for not doing so.
//!
//! # Failures are HTTP's own, where HTTP is still there
//!
//! On the verbs and the delivery routes, a container that cannot
//! serve an ask answers with a non-2xx status and a JSON body —
//! there is no error variant riding the response shapes, and no
//! second error vocabulary. The 2xx responses say only what
//! happened; the failures say so as failures.
//!
//! The run itself has one HTTP moment, the upgrade: a container
//! refuses it only for what it can know before reading the request
//! — a second run on a container that serves one (`409`), an
//! upstream that failed to install (`500`). After the upgrade there
//! is no status left to set, so a run that cannot start — the wrong
//! agent kind, a prompt it cannot speak, a continuation that will
//! not open, an upstream that will not launch — says so in the
//! loop's own vocabulary, a fatal `notification` chunk, and closes
//! the socket. One failure vocabulary on the stream, not two.
//!
//! # The response is the fate, whenever it comes
//!
//! An enqueue's HTTP response does not arrive when the message is
//! ACCEPTED — it arrives when the message's fate is known: taken
//! into the conversation, withdrawn, or outlived by the run. That
//! can be much later than the ask, and nothing times it out —
//! nothing in this protocol times anything out. The relaying server
//! holds the caller's channel open for exactly as long as the
//! container holds this response open; the two are the same wait,
//! one hop apart.
//!
//! # Same shape, same behavior
//!
//! The shapes come with obligations, and a container that serves
//! them owes all of it: the queue never interrupts the turn in
//! flight; a delivered message is marked in the response stream by a
//! [`user`](crate::endpoints::agentic_loop::run::server::response::UserChunk)
//! chunk carrying the prompt verbatim at the position it landed; a
//! dequeue answers every pending enqueue as dequeued and delivery is
//! never undone; and a message the run outlives is missed, not
//! errored.

pub mod continuation;
pub mod dequeue;
pub mod enqueue;
pub mod request;
pub mod resource;
pub mod response;
