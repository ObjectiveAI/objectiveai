//! What `diverge daemon` and `diverge provider` take.

use clap::Subcommand;

use super::{Action, Lifecycle};

/// A server on this host — the daemon or the provider — started,
/// stopped, or its document in the root worked on.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Server {
    /// Start or stop the server: see [`lifecycle`](crate::lifecycle).
    #[command(flatten)]
    Lifecycle(Lifecycle),
    /// Read, rewrite or edit the server's document in the root:
    /// `daemon.yaml` or `provider.yaml`.
    #[command(subcommand)]
    Config(Action),
}
