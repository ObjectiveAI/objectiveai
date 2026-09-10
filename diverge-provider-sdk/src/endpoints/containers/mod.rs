//! Containers: one substrate, and the two things a caller does with
//! it.
//!
//! A provider runs a container for a caller — an image, limits, the
//! caller's mounts — and the caller then works with what runs inside.
//! Two families, told apart by what that is:
//!
//! - [`agents`]: an agentic loop. The request that makes the
//!   container carries the agent — a JSON value the image defines,
//!   fixed for the container's life — and a `run_loop` channel,
//!   carrying a prompt, runs one loop and reads its chunks back; an
//!   `agent_schema` channel says
//!   what the agent value may be, and `enqueue` and `dequeue` add to
//!   the running loop's queue and clear it.
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
//! # The main stream is quiet
//!
//! A run answers with the container's
//! [`Id`](crate::shared::containers::response::Id) and then nothing,
//! for as long as the container runs; a connect answers with nothing
//! at all. An error ends the scope. Everything that used to ride
//! channel `0` — a filetree, a loop's chunks — is a channel the caller
//! opens, so a caller that wants none of it pays for none of it, and
//! two callers on one container can each subscribe to what they want.
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

pub mod agents;
pub mod tools;

#[cfg(feature = "client")]
pub mod client;
