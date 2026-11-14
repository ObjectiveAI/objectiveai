//! What `diverge identities` takes.

use clap::Subcommand;

/// The host's OpenID Connect identities under the root,
/// `<root>/identities/`, worked on here: signed in, listed, removed.
/// No daemon is dialled, and no command prints a token or a key. Each
/// identity's key on this host is made by itself, at its login or the
/// first time a token is asked for, and no command makes one.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Identities {
    /// Sign an identity in, by the device authorization grant: a code
    /// printed, and approved at the issuer. The command ends once the
    /// code is printed, and the wait for its approval goes on apart
    /// from it: the identity's lock is held for the whole login, and
    /// the identity is written once the issuer grants it — replacing
    /// one already there, signed out or not — which `identities list`
    /// then shows. A grant as another subject writes nothing.
    Login {
        /// The issuer, exactly as its discovery document names it.
        #[arg(long)]
        issuer: String,
        /// The subject to sign in as: the `sub` the approval must name.
        #[arg(long)]
        subject: String,
        /// The public client the CLI signs in as at the issuer.
        #[arg(long)]
        client_id: String,
        /// The scope asked for, space-separated; `openid` and
        /// `offline_access` are added where it lacks them. Absent,
        /// those two alone.
        #[arg(long)]
        scope: Option<String>,
        /// What the access token is for, RFC 8707's `resource`. Absent,
        /// the issuer's own choice.
        #[arg(long)]
        resource: Option<String>,
    },
    /// Every identity on the host, as `{"issuer","subject"}`, one JSON
    /// line each.
    List,
    /// Take an identity out, and its key with it.
    Remove {
        /// The issuer.
        #[arg(long)]
        issuer: String,
        /// The subject.
        #[arg(long)]
        subject: String,
    },
}
