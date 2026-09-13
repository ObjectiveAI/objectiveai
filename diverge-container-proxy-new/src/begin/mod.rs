//! The begin scopes: the server's first act on the connection, one
//! per family, and the scope the proxy's own asks ride.
//!
//! [`agents()`] registers the agent with the agent's own server, answers
//! `Begun`, starts the queue's driver, and serves the family's
//! channels — the schema, the queue's two verbs, the server's half of
//! each database connection — for the connection's life. [`tools()`]
//! answers `Begun` at once and serves the exchanges with the tool
//! container's MCP server. Both publish the scope to every surface
//! inside the container, which is the moment the container's own
//! asks can go out.

mod agents;
mod family;
mod tools;

pub use agents::*;
pub use family::*;
pub use tools::*;
