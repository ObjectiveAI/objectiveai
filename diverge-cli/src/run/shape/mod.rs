//! The exchange shapes, each run generically over an endpoint's own
//! request and answer types.
//!
//! The daemon's wire has five shapes of exchange — one answer, a
//! stream, a pending answer, an upload with content channels, and the
//! MCP connect scope — and every endpoint is one of them.
//! [`one_shot`], [`stream`] (with a [`Stop`] that ends a list at its
//! `listed` and a filetree at its snapshot), [`pending`], [`download()`],
//! [`upload`] and [`mcp()`] (the leaf naming the exchange the scope is
//! held for) put a request of that shape — decoded
//! already as the endpoint's own type, by the leaf that owns it — and
//! print its answers as the endpoint's own frames. The two shapes that carry
//! bytes go through the [`Download`] and [`Upload`] traits, one
//! implementation per endpoint, since those endpoints' executors are
//! not one function. A failure is the executor's own, in its words,
//! but for the one every shape tells apart: the connection closing
//! with no answer, which is how the daemon refuses an identity —
//! [`exchange`] names it.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod answer;
mod one_shot;
mod pending;
mod stream;
mod upload;

pub mod download;
pub mod exchange;
pub mod mcp;

pub use answer::*;
pub use download::{Download, Downloaded, download};
pub use mcp::mcp;
pub use one_shot::*;
pub use pending::*;
pub use stream::*;
pub use upload::*;
