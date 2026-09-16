//! What a container scope is made of, whichever kind it is.
//!
//! [`containers`](crate::endpoints::containers) has two families and
//! three scopes — an agent container's run, a tool container's run
//! and connect — and the three differ in one exchange apiece.
//! Everything else — asking for a container, reading and writing its
//! files, watching its tree, the connections and asks the container
//! makes back toward the caller — is the same wire in all three, so
//! it is defined once here and each scope's frames wrap or alias it.
//!
//! [`request`] is the part of asking for a container that does not
//! vary between the kinds: the image, the limits, the mounts; and the
//! other way to get one, by id. What a run answers with is
//! [`response`]: that id. The rest is what happens once a container
//! exists, split by who asks.
//!
//! The CALLER asks the container: [`read`] one file out,
//! [`write_path`] one file in — its content on a channel the provider
//! opens, [`write_bytes`] — [`transfer`] one file into another
//! container it is running or connected to, and [`filetree`] for its
//! filesystem.
//!
//! The CONTAINER asks the caller, through the provider: [`postgres`]
//! carries each database connection it opens, [`command`] a command
//! it wants run, [`vault`] the keys it keeps with the caller, [`fuse`]
//! the files the caller mounted live, and the exchanges in
//! [`mcp`](crate::shared::mcp) its tool calls outward.
//!
//! The PROVIDER asks the caller on its own account: [`oci`] for the
//! manifest and blobs of an image the caller holds, [`fetch_file`] and
//! [`fetch_directory`] for mounted content it does not hold, and
//! [`authorize`] whether a connector may join.
//!
//! [`agent_schema`], [`enqueue`] and [`dequeue`] are the agents
//! family's own exchanges — its agent's schema, and the two verbs
//! against its queue — here beside the rest of the wire they ride.
//! What the agent says is no exchange: it rides the run scope's own
//! main stream, and its chunks are defined beside that stream, in
//! [`agents::run::server::response`](crate::endpoints::containers::agents::run::server::response).
//!
//! [`filetree`](crate::shared::filetree) and [`mcp`](crate::shared::mcp)
//! stay beside this module rather than inside it: each is ridden by
//! something that is not a container scope — the proxy inside the
//! container, whose own channels carry both.

pub mod agent_schema;
pub mod authorize;
pub mod command;
pub mod dequeue;
pub mod enqueue;
pub mod fetch_directory;
pub mod fetch_file;
pub mod filetree;
pub mod fuse;
pub mod oci;
pub mod postgres;
pub mod read;
pub mod request;
pub mod response;
pub mod transfer;
pub mod vault;
pub mod write_bytes;
pub mod write_path;
