//! What an agent's create and a tool's create share.
//!
//! An agent container and a tool container are made of the same
//! things: a template, the provider they run on with its volumes,
//! the FUSE mounts of providers' volumes, the daemon's own tools they
//! hold and how far each reaches, and the agent that deploys their
//! declared dependencies. [`Inner`] is those, written once, and each
//! family's create flattens it into its own request beside the one
//! thing that is the family's — the name — so that the two requests
//! are the same shape member for member on the wire, and a reader of
//! one has read the other.

mod inner;

pub use inner::*;
