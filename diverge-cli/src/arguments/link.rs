//! What `diverge link` takes.

use clap::Subcommand;

/// What to link the daemon to.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Link {
    /// Link the daemon to the provider on this root: a key minted
    /// into the keyring, the provider told to accept it, and the
    /// daemon told to dial the provider presenting it. The provider
    /// reads `provider.yaml` at its start, so it is restarted after.
    Provider {
        /// The link's name: the key is the keyring's entry
        /// `provider/<name>`.
        #[arg(long, default_value = "local")]
        name: String,
        /// Who the daemon is to the provider: the identity the
        /// provider gives the connection.
        #[arg(long, default_value = "daemon")]
        identity: String,
        /// The daemon, by its name in `cli.yaml`. Absent, the
        /// document's default.
        #[arg(long)]
        daemon: Option<String>,
        /// The account, by its name under the daemon. Absent, the
        /// daemon's default.
        #[arg(long)]
        account: Option<String>,
    },
}
