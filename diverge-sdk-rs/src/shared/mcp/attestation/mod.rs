//! What the daemon attests under `_meta`, as types: written by the
//! daemon, read by the container on the other side.
//!
//! One object under [`KEY`](super::KEY), tagged `kind`: the
//! [`Caller`] on a request, the [`Tool`] on an answer — one or the
//! other, which side the `_meta` is on saying which, so there is no
//! union to read and no ambiguity.
//!
//! The caller is who the daemon established, never who the request
//! says: from inside the harness, the [`Agent`] whose container sent
//! the call, with its image and its key (`agent`); from outside, a
//! client reaching the tool over a connect, by nothing but the
//! identity the daemon authorized its connection as (`unbrokered`) —
//! stamped by the serving daemon on every exchange before it reaches
//! the tool, in place of whatever the connector stamped. The tool is
//! this daemon's own word on everything the tool answers: every kind
//! with the image its container was made from and the template it was
//! made from; a created tool and a connected tool one shape, told
//! apart by the kind; what this daemon cannot see of a connected
//! tool — its image, its template — the daemon that runs it told
//! once, in the answer that opened the connect, and this daemon
//! attests it as its own stamp, replacing whatever that daemon
//! stamped. A stamp from outside is never trusted, either way.
//!
//! The two sides read differently. A tool container receives the
//! four requests, each carrying the caller: [`request`] reads a
//! request's `_meta` as the [`Caller`], and is for tools in tool
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
mod attested;
mod caller;
mod error;
mod request;
mod response;
mod tool;

pub use agent::*;
pub use attested::*;
pub use caller::*;
pub use error::*;
pub use request::*;
pub use response::*;
pub use tool::*;
