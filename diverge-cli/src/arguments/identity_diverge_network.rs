//! What `diverge identity.diverge.network` takes.

use clap::Subcommand;

/// `identity.diverge.network`, Diverge's own OpenID Connect provider,
/// worked on from this host.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum IdentityDivergeNetwork {
    /// The host's accounts there: each a user of an external provider
    /// — GitHub, Google — which owns every subject that begins with
    /// its provider and its id.
    #[command(subcommand)]
    Accounts(Accounts),
}

/// The host's accounts at `identity.diverge.network`, under the root.
/// No command prints a token.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Accounts {
    /// Every account on the host, as `{"account","provider","id"}`,
    /// one JSON line each.
    List,
    /// Sign in to an account, at its provider, by a device code: the
    /// code printed, the provider's page opened in this host's
    /// browser, and the command ended there. The wait for the code's
    /// approval goes on apart from it, and the account is written once
    /// the provider grants it — replacing one already there — which
    /// `accounts list` then shows.
    Login {
        /// The provider to sign in at, by its name: `github`,
        /// `google`. A name that is none is refused with the ones
        /// there are.
        provider: String,
        /// Open no browser: the line printed names the page, for a
        /// person to open.
        #[arg(long)]
        no_browser: bool,
    },
}
