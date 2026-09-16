//! Judging an unbrokered credential: the SDK's `UnbrokeredAuthorizer`,
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
//! close and reaches the provider's log with the hooks that did not
//! answer, by name, so a broken hook is seen as broken.
//!
//! [`UnbrokeredAuthorizer`] is the authorizer and [`Error`] the
//! refusal.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod unbrokered_authorizer;

pub use error::*;
pub use unbrokered_authorizer::*;
