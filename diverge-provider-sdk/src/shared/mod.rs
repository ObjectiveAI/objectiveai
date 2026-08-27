//! Shapes that more than one endpoint is made of.
//!
//! Nothing here is something a client can ask for. An
//! [`endpoint`](crate::endpoints) is a scope somebody opens; these are
//! the pieces several of them turn out to be built from, factored out
//! so that one definition serves all of them.
//!
//! [`mcp`] is the five exchanges an MCP server answers, each one typed
//! and each one its own channel. [`oci`] is the OCI Distribution
//! protocol, which is bytes: a caller serving its own image answers
//! what the runtime asked, verbatim, and nothing between them reads it.
//! [`fetch`] is content by identity — a skill or an agent definition
//! the provider is missing, asked for by dirhash and answered as one
//! frame per file.
//!
//! They were one module until recently — a tunneled HTTP exchange that
//! both rode. Removing it lost nothing, because MCP over HTTP is
//! JSON-RPC with a transport under it and the transport was the only
//! part being carried; and it took with it every bug that came of
//! rebuilding a request rather than forwarding one.
//!
//! [`filetree`] is a live filesystem view — a watch answers with one,
//! and so does a laboratory run, over a different tree. [`container`]
//! is what a container endpoint does to a container once it has one,
//! plus the part of asking for one that does not vary between the
//! kinds.
//!
//! [`error`] is the odd one out: a shape nothing carries yet. It is
//! here rather than beside whichever frame first needs it, because a
//! failure that means different things in different modules is a
//! failure every consumer has to learn twice.
//!
//! The reason they live here rather than in whichever endpoint used
//! them first: a shape defined twice is two shapes that agree until
//! they do not, and the fold in
//! [`Root::update`](filetree::response::Root::update) is exactly the
//! kind of thing that would stop agreeing quietly.

pub mod container;
pub mod error;
pub mod fetch;
pub mod filetree;
pub mod mcp;
pub mod oci;
