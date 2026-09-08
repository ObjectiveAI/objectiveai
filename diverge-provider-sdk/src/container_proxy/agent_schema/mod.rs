//! The `/agent-schema` path: what the agent value may be.
//!
//! Opened by the server, for an agent container, carrying nothing —
//! the opening is the ask. The container answers with one
//! [`response::Frame`]: the JSON Schema of the `agent` value its
//! image accepts, or an `Error` for an image that posted none. Then
//! the close.
//!
//! ```text
//! container → server:   [0][schema JSON] | [1][error JSON]    then the close
//! ```
//!
//! The schema is the harness's: it posts one at `/agent-schema/agent`
//! when it starts, and the proxy keeps the latest and answers every
//! opening with it. An image whose harness posts nothing is answered
//! `Error`, at once — a schema is a courtesy an image extends, not an
//! obligation, and a server that wants one and gets none knows so.

pub mod response;

#[cfg(feature = "server")]
pub mod execute;
