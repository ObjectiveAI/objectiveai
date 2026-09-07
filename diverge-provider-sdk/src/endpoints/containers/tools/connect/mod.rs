//! Joining a tool container somebody else is running.
//!
//! Split by who SENDS: [`client`] is the caller's traffic, [`server`]
//! the provider's.
//!
//! Three parties, and the provider is the middle one. A connector
//! names a container by the id its runner was given and offers an
//! authorization; the provider asks the runner, on the run scope,
//! whether that is enough; and the scope opens on a yes. From then on
//! the connector reaches into the container exactly as the runner
//! does — files, the tree, the family's own exchange — and the scope
//! ends when the connector leaves or the container does.
//!
//! What a connector does NOT do is supply anything on the container's
//! account. The image was somebody else's problem, and so are the
//! container's own asks — its database, its commands, its vault, its
//! tools — which go to whoever runs it. The one channel a provider
//! opens on a connector is for the content of a write the connector
//! itself started.

pub mod client;
pub mod server;
