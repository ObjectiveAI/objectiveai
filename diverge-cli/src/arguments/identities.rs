//! What `diverge identities` takes.

use clap::Subcommand;

/// The host's OpenID Connect identities under the root,
/// `<root>/identities/`, worked on here: signed in, listed, removed,
/// and given keys. No daemon is dialled, and no command prints a token
/// or a private key.
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
    /// An identity's key on this host: the key its tokens name, which
    /// every connection made as the identity is signed with.
    #[command(subcommand)]
    Key(Key),
}

/// What `diverge identities key` takes.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Key {
    /// Make a key for an identity and print its public half, as a
    /// JSON Web Key. An identity that has a key already keeps it, and
    /// the command fails, unless `--overwrite` is given: the old key
    /// is then gone, every token that named it with it, and the
    /// identity's next token names the new one. A login makes a key by
    /// itself where the identity has none, so this is for replacing
    /// one, or for making one before the login.
    Generate {
        /// The issuer.
        #[arg(long)]
        issuer: String,
        /// The subject.
        #[arg(long)]
        subject: String,
        /// Replace the key the identity already has.
        #[arg(long)]
        overwrite: bool,
    },
}
