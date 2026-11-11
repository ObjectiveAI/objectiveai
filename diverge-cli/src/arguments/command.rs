//! What `diverge` does.

use clap::Subcommand;

use super::{Document, Identities, Issuer, Server};
use crate::run::Run;

/// The commands. Every request of the daemon's wire goes under
/// [`Run`](Self::Run). Each program is its own command, and its
/// document in the root is worked on beneath it, as `config`: the
/// daemon and the provider on this host are also started and stopped
/// under [`Daemon`](Self::Daemon) and [`Provider`](Self::Provider); the
/// issuer, which takes no commands of its own, is administered under
/// [`Issuer`](Self::Issuer); the viewer and the CLI itself are
/// [`Viewer`](Self::Viewer) and [`Cli`](Self::Cli), configured alone. The host's identities are
/// [`Identities`](Self::Identities).
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Command {
    /// Put one request to a daemon, as an account of it: see
    /// [`run`](crate::run).
    Run(Run),
    /// Sign in, list or remove an OpenID Connect identity under the
    /// root, the one store of identities every Diverge program on the
    /// host shares. No command prints a token.
    #[command(subcommand)]
    Identities(Identities),
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
    /// Start or stop the issuer on the root, work on its
    /// `issuer.yaml`, its identities and its signing key: see
    /// [`issuer`](crate::cli::issuer).
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
