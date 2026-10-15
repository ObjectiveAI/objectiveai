//! What `diverge` does.

use clap::Subcommand;

use super::Config;
use crate::run::Run;

/// The commands. Every request of the daemon's wire goes under
/// [`Run`](Self::Run); the documents in the root are
/// [`Config`](Self::Config)'s.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Command {
    /// Put one request to a daemon, as an account of it: see
    /// [`run`](crate::run).
    Run(Run),
    /// Read, rewrite or edit one of the three documents in the root.
    Config(Config),
}
