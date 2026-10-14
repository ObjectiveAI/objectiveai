//! What the daemon attests under `_meta`, as types: written by the
//! daemon, read by the container on the other side.
//!
//! One object under [`KEY`](super::KEY), an [`Attestation`], tagged
//! `kind`: an [`Agent`] (`agent`), or a [`Tool`] — `created`,
//! `connected` or `dependency`. Every kind carries the image its
//! container was made from, and every tool kind the template too: a
//! reader that holds the type holds them. A created tool and a
//! connected tool are one shape, told apart by the kind alone; what
//! this daemon cannot see of a connected tool — its image, its
//! template — the daemon that runs it told once, in the answer that
//! opened the connect, and this daemon attests it as its own stamp,
//! replacing whatever that daemon stamped: a stamp from outside is
//! not trusted. The other way, a request that arrives over a connect
//! from another daemon's agent is put to the tool as that daemon sent
//! it — this daemon stamps nothing on it, there being no point.
//!
//! The two sides read differently. A tool container receives the
//! four requests, each carrying the calling agent: [`request`] reads
//! a request's `_meta` as the [`Agent`], and is for tools in tool
//! containers alone. An agent container receives the answers — each
//! result, each entry of a list, each notification — each carrying
//! the answering tool: [`response`] reads an answer's `_meta` as the
//! [`Tool`], and is for agents in agent containers alone. Either read
//! refuses a `_meta` the daemon did not attest that way, with the
//! [`Error`].
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod agent;
mod attestation;
mod error;
mod request;
mod response;
mod tool;

pub use agent::*;
pub use attestation::*;
pub use error::*;
pub use request::*;
pub use response::*;
pub use tool::*;
