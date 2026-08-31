//! The HTTP surface every agentic-loop container serves.
//!
//! One definition, whatever the upstream behind it: a container
//! takes THE request ([`request::Request`]) at `POST /` on the loop
//! port — `8080`, per the Container section of the provider
//! specification — and answers with a server-sent event stream whose
//! events are [`response::Response`] items: the loop's chunks, and —
//! when the request named resources — the container's own
//! [`fetch_resource`](response::FetchResource) asks, which the
//! server consumes and answers by POSTing the bytes back in at
//! `POST /resource/{identity}` ([`resource`]), chunked, settled by
//! the completion — or by the error, when the bytes can never
//! come.
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
//! # Failures are HTTP's own
//!
//! A container that cannot serve an ask answers with a non-2xx
//! status and a JSON body, the same form the run request already
//! answers with — there is no error variant riding the response
//! shapes, and no second error vocabulary. The 2xx responses say
//! only what happened; the failures say so as failures.
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

pub mod dequeue;
pub mod enqueue;
pub mod request;
pub mod resource;
pub mod response;
