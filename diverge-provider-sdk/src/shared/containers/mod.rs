//! What a container scope is made of, whichever kind it is.
//!
//! [`containers`](crate::endpoints::containers) has two families and
//! each has two scopes, and the four differ in one exchange apiece.
//! Everything else — asking for a container, reading and writing its
//! files, watching its tree, the connections and asks the container
//! makes back toward the caller — is the same wire in all four, so it
//! is defined once here and each scope's frames wrap or alias it.
//!
//! [`request`] is the part of asking for a container that does not
//! vary between the kinds: the image, the limits, the mounts; and the
//! other way to get one, by id. What a run answers with is
//! [`response`]: that id. The rest is what happens once a container
//! exists, split by who asks.
//!
//! The CALLER asks the container: [`read`] one file out,
//! [`write_path`] one file in — its content on a channel the provider
//! opens, [`write_bytes`] — and [`filetree`] for its filesystem.
//!
//! The CONTAINER asks the caller, through the provider: [`postgres`]
//! carries each database connection it opens, [`command`] a command
//! it wants run, [`vault`] the keys it keeps with the caller, and the
//! exchanges in [`mcp`](crate::shared::mcp) its tool calls outward.
//!
//! The PROVIDER asks the caller on its own account: [`oci`] for the
//! manifest and blobs of an image the caller holds, [`fetch_file`] and
//! [`fetch_directory`] for mounted content it does not hold, and
//! [`authorize`] whether a connector may join.
//!
//! [`agentic_loop`] and [`agent_schema`] are the agents family's own
//! exchanges; they are here rather than in that family because its
//! run and its connect both carry them.
//!
//! [`filetree`](crate::shared::filetree) and [`mcp`](crate::shared::mcp)
//! stay beside this module rather than inside it: each is ridden by
//! something that is not a container scope — a volume watch, the
//! proxy inside the container.

pub mod agent_schema;
pub mod agentic_loop;
pub mod authorize;
pub mod command;
pub mod fetch_directory;
pub mod fetch_file;
pub mod filetree;
pub mod oci;
pub mod postgres;
pub mod read;
pub mod request;
pub mod response;
pub mod vault;
pub mod write_bytes;
pub mod write_path;
