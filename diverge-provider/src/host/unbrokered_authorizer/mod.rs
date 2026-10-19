//! Judging an unbrokered credential: the wire's `UnbrokeredAuthorizer`,
//! supplied by this crate.
//!
//! A peer that dials the provider presents a credential first, and
//! this decides whether it may connect and who it is, from the `auth`
//! section of the configuration. Each way the section lists judges
//! on its own: a key is a string the credential must equal, byte for
//! byte, presented from one address when the key names one, and the
//! peer that presents it is the key's identity; a hook is a program
//! under `hooks/<name>/` handed the credential and the address, which
//! answers with an identity or with a refusal, and a hook that does
//! not answer refuses. Every way is asked at once — a key is answered
//! in the time of a comparison, a hook in the time of a process — and
//! the first, in the configuration's order, that accepts decides, so
//! the peer waits for the slowest hook and not for the sum of them,
//! and the answer never depends on which finished first. No way
//! accepting is a refusal, which reaches the peer as nothing but the
//! close, and carries the hooks that did not answer, by name, so a
//! broken hook is seen as broken by whoever holds the error.
//!
//! A credential accepted is then held: the identity it admits and
//! the credential itself are taken in the provider's
//! [`Peers`](crate::host::serve::Peers) for the connection's life, and a
//! connection that collides on either is refused the same way a bad
//! credential is. What was taken is left in the connection's
//! [`Slot`] as [`Taken`], for the connection to give back.
//!
//! [`UnbrokeredAuthorizer`] is the authorizer and [`Error`] the
//! refusal.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod taken;
mod unbrokered_authorizer;

pub use error::*;
pub use taken::*;
pub use unbrokered_authorizer::*;
