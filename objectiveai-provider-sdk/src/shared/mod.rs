//! Shapes that more than one endpoint is made of.
//!
//! Nothing here is something a client can ask for. An
//! [`endpoint`](crate::endpoints) is a scope somebody opens; these are
//! the pieces several of them turn out to be built from, factored out
//! so that one definition serves all of them.
//!
//! [`http`] is a tunneled HTTP exchange — MCP rides it in an agentic
//! loop and in a container, and the OCI Distribution protocol rides it
//! when a caller serves its own image. [`filetree`] is a live
//! filesystem view — a watch answers with one, and so does a container
//! creation, over a different tree. [`container`] is what both
//! container endpoints do to a container once they have one.
//!
//! The reason they live here rather than in whichever endpoint used
//! them first: a shape defined twice is two shapes that agree until
//! they do not, and the fold in
//! [`Root::update`](filetree::response::Root::update) is exactly the
//! kind of thing that would stop agreeing quietly.

pub mod container;
pub mod filetree;
pub mod http;
