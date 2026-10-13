//! The exchange shapes, each run generically over an endpoint's own
//! request and answer types.
//!
//! The daemon's wire has five shapes of exchange — one answer, a
//! stream, a pending answer, an upload with content channels, and the
//! MCP connect scope — and every endpoint is one of them.
//! [`one_shot`], [`stream`] (with a [`Stop`] that ends a list at its
//! `listed` and a filetree at its snapshot), [`pending`], [`download`],
//! [`upload`] and [`mcp`] put a request of that shape and print its
//! answers: the request decoded as the
//! endpoint's own type from the bytes, so that a body the endpoint
//! does not take is refused in the endpoint's words; the answers
//! emitted as the endpoint's own frames. The two shapes that carry
//! bytes go through the [`Download`] and [`Upload`] traits, one
//! implementation per endpoint, since those endpoints' executors are
//! not one function. A failure is the executor's own, in its words,
//! but for the one every shape tells apart: the connection closing
//! with no answer, which is how the daemon refuses a credential —
//! [`exchange`] names it.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod download;
mod mcp;
mod one_shot;
mod pending;
mod stream;
mod upload;

pub mod exchange;

pub use download::*;
pub use mcp::*;
pub use one_shot::*;
pub use pending::*;
pub use stream::*;
pub use upload::*;

use diverge_sdk::wire::decode::Decode;

use super::Error;

/// The request as the endpoint's own type, from the bytes; the
/// endpoint's own refusal when the body is not what it takes.
pub fn request<Q: for<'a> Decode<'a>>(bytes: &[u8]) -> Result<Q, Error> {
    Q::decode(bytes).map_err(|error| Error::Request(Box::new(error)))
}
