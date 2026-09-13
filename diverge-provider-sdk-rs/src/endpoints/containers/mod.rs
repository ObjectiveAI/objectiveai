//! Containers: one substrate, and the two things a caller does with
//! it.
//!
//! A provider runs a container for a caller — an image, limits, the
//! caller's mounts — and the caller then works with what runs inside.
//! Two families, told apart by what that is:
//!
//! - [`agents`]: a conversation. The request that makes the
//!   container carries the agent — a JSON value the image defines,
//!   fixed for the container's life — and the agent's chunks ride the
//!   scope's own main stream after the id; an `enqueue` channel sends
//!   it a message, starting a loop when none runs and queueing one
//!   when it does, `dequeue` clears what is waiting, and
//!   `agent_schema` says what the agent value may be.
//! - [`tools`]: an MCP server. The caller opens the five MCP exchanges
//!   into it.
//!
//! Everything else is identical, and it is most of the wire: each
//! family has a `run` that owns the container's life, and the tools
//! family also has a `connect` that joins one by id and authorization
//! — an agent container is its runner's alone; every scope reads and
//! writes files, watches the tree, and relays what the container asks
//! of the caller — its database connections, its commands, its vault,
//! its tool calls outward. All of that is defined once, in
//! [`shared::containers`](crate::shared::containers), and each scope's
//! frames wrap or alias it. The three scopes' channel tags are laid out
//! so the shared part comes first and identically, and the family's
//! own exchange takes the tags after it.
//!
//! # The main stream is the id, and then the agent
//!
//! A run answers with the container's
//! [`Id`](crate::shared::containers::response::Id). An agent
//! container's stream then carries the agent's chunks for as long as
//! the container runs; a tool container's carries nothing more, and a
//! connect answers with nothing at all. An error ends the scope.
//! Everything else a caller reads — a filetree, a file — is a channel
//! the caller opens, so a caller that wants none of it pays for none
//! of it, and two callers on one tool container can each subscribe
//! to what they want.
//!
//! # Why one family is not enough
//!
//! An agent and a tool server are both a container, and the wire
//! treats them as one until the caller reaches in. They are two
//! families rather than one with a flag because the reaching in is
//! typed — a loop's request and its chunks, an MCP exchange and its
//! result — and a single channel-request enum carrying both would
//! carry variants that can never be valid on half the scopes it
//! serves.
//!
//! # And, behind the `client` feature, a way to use them
//!
//! Each scope's `client::execute` performs the exchange rather than
//! describing it: hand it a [`Handle`](crate::client::handle::Handle),
//! the request and — for a run — the caller's
//! [`Answerers`](crate::client::Answerers), and get back the
//! container's id and a handle that holds the scope for the
//! container's life, opens every channel the caller may open, and
//! answers every channel the provider opens. What the three share is
//! [`client`], written once.
//!
//! # And, behind the `server` feature, a way to serve them
//!
//! Each scope's `server::handle` answers the request: a run brings
//! the container up in order — content, registry, deploy, the proxy
//! dialled and begun, every mount made — sends the id, and then
//! relays everything the container asks and serves everything the
//! caller opens until the run ends; a connect asks the runner and, on a yes,
//! serves the connector the same way. What the three share is
//! [`server`], written once.

pub mod agents;
pub mod tools;

#[cfg(any(feature = "client", feature = "server"))]
pub mod client;

#[cfg(feature = "server")]
pub mod server;
