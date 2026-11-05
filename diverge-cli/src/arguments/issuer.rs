//! What `diverge issuer` takes.

use clap::Subcommand;

use super::{Action, Lifecycle};

/// The issuer on this host: started, stopped, its `issuer.yaml` worked
/// on, the device codes waiting decided, the identities in its
/// `issuer.sqlite`, and its signing key rotated.
/// The issuer takes no commands of its own: everything but serving is
/// done here. The identities and the key are written to the database
/// directly, whether the issuer runs or not; a device code is decided
/// by the running issuer, since a code only waits while it runs.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Issuer {
    /// Start or stop the issuer: see [`lifecycle`](crate::lifecycle).
    #[command(flatten)]
    Lifecycle(Lifecycle),
    /// Read, rewrite or edit the issuer's `issuer.yaml`.
    #[command(subcommand)]
    Config(Action),
    /// The identities tokens have been issued to.
    #[command(subcommand)]
    Identities(IssuerIdentities),
    /// Every device code waiting for the operator, the soonest to
    /// expire first: its user code, the subject it asks for, and when it
    /// expires, one JSON line each. The issuer must be running.
    Pending,
    /// Approve a device code as the subject it asks for, which becomes
    /// an identity then if it was not one: the login waiting on it is
    /// signed in. The issuer must be running.
    Approve {
        /// The user code, as the login printed it.
        code: String,
    },
    /// Deny a device code: the login waiting on it ends denied. The
    /// issuer must be running.
    Deny {
        /// The user code, as the login printed it.
        code: String,
    },
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

/// What is done to the identities. An identity comes to be when a code
/// asking for it is approved; the issuer reads them at every request,
/// so a change holds at once.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum IssuerIdentities {
    /// Every identity, by subject, with when it first was approved: one
    /// JSON line each.
    List,
    /// Take an identity out, and every device code and refresh token
    /// issued to it.
    Remove {
        /// The subject.
        subject: String,
    },
}
