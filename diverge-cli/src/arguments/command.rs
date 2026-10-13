//! What `diverge` does.

use clap::Subcommand;

/// The commands. Every request of the daemon's wire goes under
/// [`Run`](Self::Run); this cut declares it and serves nothing under
/// it.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Command {
    /// Put a request to the daemon. Not yet served: the configuration
    /// is resolved and the program ends.
    Run,
}
