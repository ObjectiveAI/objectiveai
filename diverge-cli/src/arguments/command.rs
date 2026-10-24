//! What `diverge` does.

use clap::Subcommand;

use super::{Config, Issuer, Keyring, Lifecycle, Link};
use crate::run::Run;

/// The commands. Every request of the daemon's wire goes under
/// [`Run`](Self::Run); the documents in the root are
/// [`Config`](Self::Config)'s; the daemon and the provider on this
/// host are started and stopped under [`Daemon`](Self::Daemon) and
/// [`Provider`](Self::Provider), and the issuer under
/// [`Issuer`](Self::Issuer), which also works on its identities and
/// clients.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Command {
    /// Put one request to a daemon, as an account of it: see
    /// [`run`](crate::run).
    Run(Run),
    /// Read, rewrite or edit one of the five documents in the root.
    Config(Config),
    /// List, mint, set or remove a key in the root's keyring, the one
    /// store of keys every Diverge program on the host shares. No
    /// command prints a key.
    #[command(subcommand)]
    Keyring(Keyring),
    /// Link the daemon to a provider: the key minted, the provider
    /// told to accept it, the daemon told to dial presenting it.
    #[command(subcommand)]
    Link(Link),
    /// Start or stop the daemon on the root: see
    /// [`lifecycle`](crate::lifecycle).
    #[command(subcommand)]
    Daemon(Lifecycle),
    /// Start or stop the provider on the root: see
    /// [`lifecycle`](crate::lifecycle).
    #[command(subcommand)]
    Provider(Lifecycle),
    /// Start or stop the issuer on the root, or work on its
    /// identities and clients: see [`issuer`](crate::cli::issuer).
    #[command(subcommand)]
    Issuer(Issuer),
}
