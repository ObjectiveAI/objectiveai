//! What `diverge` does.

use clap::Subcommand;

use super::{Document, Issuer, Keyring, Link, Server};
use crate::run::Run;

/// The commands. Every request of the daemon's wire goes under
/// [`Run`](Self::Run). Each program is its own command, and its
/// document in the root is worked on beneath it, as `config`: the
/// daemon and the provider on this host are also started and stopped
/// under [`Daemon`](Self::Daemon) and [`Provider`](Self::Provider), and
/// the issuer under [`Issuer`](Self::Issuer), which also works on its
/// identities and clients; the viewer and the CLI itself are
/// [`Viewer`](Self::Viewer) and [`Cli`](Self::Cli), configured alone.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Command {
    /// Put one request to a daemon, as an account of it: see
    /// [`run`](crate::run).
    Run(Run),
    /// List, mint, set or remove a key in the root's keyring, the one
    /// store of keys every Diverge program on the host shares. No
    /// command prints a key.
    #[command(subcommand)]
    Keyring(Keyring),
    /// Link the daemon to a provider: the key minted, the provider
    /// told to accept it, the daemon told to dial presenting it.
    #[command(subcommand)]
    Link(Link),
    /// Start or stop the daemon on the root, or work on its
    /// `daemon.yaml`: see [`lifecycle`](crate::lifecycle) and
    /// [`config`](crate::cli::config).
    #[command(subcommand)]
    Daemon(Server),
    /// Start or stop the provider on the root, or work on its
    /// `provider.yaml`: see [`lifecycle`](crate::lifecycle) and
    /// [`config`](crate::cli::config).
    #[command(subcommand)]
    Provider(Server),
    /// Start or stop the issuer on the root, or work on its
    /// identities and clients: see [`issuer`](crate::cli::issuer).
    #[command(subcommand)]
    Issuer(Issuer),
    /// Work on the viewer's `viewer.yaml`: see
    /// [`config`](crate::cli::config).
    #[command(subcommand)]
    Viewer(Document),
    /// Work on the CLI's own `cli.yaml`: see
    /// [`config`](crate::cli::config).
    #[command(subcommand)]
    Cli(Document),
}
