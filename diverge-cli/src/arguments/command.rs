//! What `diverge` does.

use clap::Subcommand;

use super::{Config, Lifecycle};
use crate::run::Run;

/// The commands. Every request of the daemon's wire goes under
/// [`Run`](Self::Run); the documents in the root are
/// [`Config`](Self::Config)'s; the daemon and the provider on this
/// host are started and stopped under [`Daemon`](Self::Daemon) and
/// [`Provider`](Self::Provider).
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Command {
    /// Put one request to a daemon, as an account of it: see
    /// [`run`](crate::run).
    Run(Run),
    /// Read, rewrite or edit one of the four documents in the root.
    Config(Config),
    /// Start or stop the daemon on the root: see
    /// [`lifecycle`](crate::lifecycle).
    #[command(subcommand)]
    Daemon(Lifecycle),
    /// Start or stop the provider on the root: see
    /// [`lifecycle`](crate::lifecycle).
    #[command(subcommand)]
    Provider(Lifecycle),
}
