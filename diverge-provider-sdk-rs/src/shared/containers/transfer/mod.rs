//! One file, from this container into another, without the caller
//! in between.
//!
//! A client opens a channel on the scope of the container the file
//! is in with a [`request::Request`] — the file, the other container
//! by the id its run answered, and the destination there — and the
//! provider answers on that same channel with a [`response::Frame`]
//! once the file is at the destination, or with the exchange's error.
//! The bytes never reach the client: the provider reads the file out
//! of the one container and writes it into the other, on its own
//! connections to their proxies, which is a
//! [`read`](super::read) wired into a [`write_path`](super::write_path)
//! and nothing new on either proxy.
//!
//! # Who may ask
//!
//! The client must be running, or connected to, BOTH containers: the
//! one the scope is on, which holding the scope already establishes,
//! and the one the request names, which the provider checks against
//! who is running it and who is attached to it. A request naming a
//! container the client is neither running nor connected to is
//! refused, and so is one naming an id under which nothing runs, with
//! the same error — a refusal that said which would tell a stranger
//! whether an id exists. Holding the id is not enough on its own: an
//! id is a capability to ASK to attach, and attaching is what a
//! connect scope is for.

pub mod request;
pub mod response;
