//! What `diverge identities` takes.

use clap::Subcommand;

/// The host's OpenID Connect identities under the root,
/// `<root>/identities/`, worked on here: signed in, listed, removed.
/// No daemon is dialled, and no command prints a token.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Identities {
    /// Sign an identity in. By default, OpenID Connect's Authorization
    /// Code flow: the browser opened at the issuer's sign-in, and its
    /// redirect taken on a port of this host's loopback. With
    /// `--device`, the device authorization grant: a code approved at
    /// the issuer's verification page, in any browser. With
    /// `--password-stdin`, the password grant, one round trip — how
    /// Diverge's own issuer signs in. The identity's lock is held for
    /// the whole login, and the identity is written once the issuer
    /// grants it — replacing one already there, signed out or not. A
    /// grant as another subject writes nothing.
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
        /// Sign in by the identity's password, read from stdin to its
        /// end, the trailing line end dropped: no code, no browser.
        #[arg(long, conflicts_with = "device")]
        password_stdin: bool,
        /// Sign in by the device authorization grant: a code printed,
        /// and approved at the issuer's verification page in any
        /// browser, this host's or another device's.
        #[arg(long)]
        device: bool,
        /// Open no browser: the line printed names the page, for a
        /// person to open.
        #[arg(long)]
        no_browser: bool,
    },
    /// Every identity on the host, as `{"issuer","subject"}`, one JSON
    /// line each.
    List,
    /// Take an identity out.
    Remove {
        /// The issuer.
        #[arg(long)]
        issuer: String,
        /// The subject.
        #[arg(long)]
        subject: String,
    },
}
