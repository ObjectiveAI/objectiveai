//! The begin scopes: the server's first act on the connection, one
//! per family, and the scope the proxy's own asks ride.
//!
//! Either begin registers the arguments with the program's own server
//! and answers `Begun` with what the program declared — the
//! dependencies, for an agent container, and for both the secrets it
//! takes, by name and mode; the values of those come after `Begun`,
//! on a secrets channel the server opens, and the container's gate
//! is closed until the first does. [`agents()`] then starts the
//! queue's driver and serves the family's channels — the schema, the
//! queue's two verbs, the feed, the server's half of each database
//! connection — for the connection's life, and [`tools()`] serves the
//! schema, the feed, and the exchanges with the tool container's MCP
//! server. Both publish the scope to every surface inside the
//! container, which is the moment the container's own asks can go
//! out; and both finish the scope when the proxy ends the run — a
//! collect of the rotating secrets that failed — since each begin's
//! loop is the only task that may.

mod agents;
mod family;
mod tools;

pub use agents::*;
pub use family::*;
pub use tools::*;
