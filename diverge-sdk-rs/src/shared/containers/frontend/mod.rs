//! A program's frontend: its own HTTP server, beneath `/frontend`.
//!
//! A tool or an agent container's program is one HTTP server on the container's
//! loopback (see [`port`](crate::container_proxy::inside::port)); an
//! image whose `/DIVERGE.md` declares `frontend` serves pages there
//! beneath `/frontend`. One exchange is one HTTP request: its
//! [`request::Head`], then its body; the answer is its
//! [`output`] — the response's head, then its body.
//!
//! The rules:
//!
//! 1. Only a path beneath `/frontend` is reached, as
//!    [`request::Head::reachable`] states: the program's other routes
//!    never are.
//! 2. The request's body is the [`input`]: any number of `Body`, then
//!    `End`.
//! 3. The output is exactly one `Head`, then any number of `Body`; or a
//!    lone `Error`.
//! 4. An answer with status `101` switches the protocol: after it, the
//!    input's `Body` carries the bytes the page sends and the output's
//!    `Body` the bytes the program sends, until either side ends.

pub mod input;
pub mod output;
pub mod request;
