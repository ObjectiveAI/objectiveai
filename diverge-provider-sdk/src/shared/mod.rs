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
//!
//! They were one module until recently — a tunneled HTTP exchange that
//! both rode. Removing it lost nothing, because MCP over HTTP is
//! JSON-RPC with a transport under it and the transport was the only
//! part being carried; and it took with it every bug that came of
//! rebuilding a request rather than forwarding one.
//!
//! [`filetree`] is a live filesystem view — a watch answers with one,
//! and so does a container scope, over a different tree.
//! [`containers`] is everything the four container scopes have in
//! common, which is everything but one exchange each: asking for a
//! container, working with its files, and the asks it makes back.
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

pub mod containers;
pub mod error;
pub mod filetree;
pub mod mcp;
pub mod oci;
