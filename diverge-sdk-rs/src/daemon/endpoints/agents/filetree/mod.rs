//! Watching an agent's container's filesystem, whole.
//!
//! A client names an agent; the daemon sends a snapshot of the
//! container's whole tree — every mount in it, the FUSE mounts the
//! daemon itself serves spliced in at their mount points — and then one
//! frame per change, for as long as the scope lives, and finishes when
//! the client cancels or the agent is deleted; or answers that no agent
//! is the one named, forbidden, or that it failed, and finishes. The
//! tree is the provider protocol's
//! [`filetree`](crate::shared::filetree) stream, which leaves the FUSE
//! mounts out, made whole by the daemon: a resource mount's subtree
//! from the bytes the daemon holds and the layer it writes into, a
//! volume mount's from the filetree channel of the `volumes::serve` the
//! daemon bridges the mount to, and a change under either from the same
//! source. An agent's container that is not running is started for the
//! watch and stopped when the scope ends; one the daemon has running
//! anyway is used as it runs.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question and the cancel are in [`client`] — and the daemon answers,
//! so the answer is in [`server`]. Neither side holds both halves of
//! the exchange.

pub mod client;
pub mod server;
