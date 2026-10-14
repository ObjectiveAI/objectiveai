//! What the daemon attested under `_meta`, read back as types.
//!
//! [`attest`](super::attest) writes three keys; this module reads
//! them, on the side that receives them. The two sides are not the
//! same side. A tool container receives the four requests, each
//! carrying the calling agent: [`request`] reads a request's `_meta`
//! into a [`Request`], and is for tools in tool containers alone. An
//! agent container receives the answers — each result, each entry of
//! a list, each notification — each carrying the answering tool:
//! [`response`] reads an answer's `_meta` into a [`Response`], and is
//! for agents in agent containers alone. Either read refuses a
//! `_meta` the daemon did not attest that way, with the [`Error`]
//! naming the key.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod request;
mod response;

pub use error::*;
pub use request::*;
pub use response::*;
