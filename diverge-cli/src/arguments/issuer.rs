//! What `diverge issuer` takes.

use clap::Subcommand;

use super::{Action, Lifecycle};

/// The issuer on this host: started, stopped, its `issuer.yaml` worked
/// on, and the identities and the signing keys in its `issuer.sqlite`.
/// The issuer takes no commands of its own: everything but serving is
/// done here. The identities and the keys are written to the database
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
    /// The keys every token is signed with.
    #[command(subcommand)]
    Keys(IssuerKeys),
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

/// What is done to the signing keys. The issuer reads them at its
/// start, so a rotation holds from its next start.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum IssuerKeys {
    /// Mint a new key and retire every other. From the issuer's next
    /// start only the new key signs and is published, so a token
    /// signed before stops verifying; a program signed in refreshes
    /// for a new one, unless `--sign-out`.
    Rotate {
        /// Revoke every refresh token as well: every identity signs in
        /// again.
        #[arg(long)]
        sign_out: bool,
    },
    /// Every key, by id, with when it was made and when it was
    /// retired: one JSON line each, never the key.
    List,
}
