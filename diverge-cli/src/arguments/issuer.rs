//! What `diverge issuer` takes.

use clap::Subcommand;

use super::{Action, Lifecycle};

/// The issuer on this host: started, stopped, its `issuer.yaml` worked
/// on, the identities in its `issuer.sqlite`, and its signing key
/// rotated.
/// The issuer takes no commands of its own: everything but serving is
/// done here. The identities and the key are written to the database
/// directly, whether the issuer runs or not.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Issuer {
    /// Start or stop the issuer: see [`lifecycle`](crate::lifecycle).
    #[command(flatten)]
    Lifecycle(Lifecycle),
    /// Read, rewrite or edit the issuer's `issuer.yaml`.
    #[command(subcommand)]
    Config(Action),
    /// The identities a person signs in as.
    #[command(subcommand)]
    Identities(IssuerIdentities),
    /// Rotate the key every token is signed with: a new key minted and
    /// the old removed. A running issuer rotates itself and signs with
    /// the new key at once; one that is not running signs with it from
    /// its start. A token signed before stops verifying; a program
    /// signed in refreshes for a new one, unless `--sign-out`.
    Rotate {
        /// Revoke every refresh token as well: every identity signs in
        /// again.
        #[arg(long)]
        sign_out: bool,
    },
}

/// What is done to the identities. The issuer reads them at every
/// request, so a change holds at once.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum IssuerIdentities {
    /// Add an identity, its password read from stdin to its end, the
    /// trailing line end dropped. A subject already held is refused.
    Add {
        /// The subject: the `sub` of every token issued to it.
        subject: String,
    },
    /// Every identity, by subject, with when it was added: one JSON
    /// line each, never a password's hash.
    List,
    /// Take an identity out, and every device code and refresh token
    /// issued to it.
    Remove {
        /// The subject.
        subject: String,
    },
    /// Change an identity's password, read from stdin to its end, and
    /// revoke every refresh token issued to it: it signs in again.
    Password {
        /// The subject.
        subject: String,
    },
}
