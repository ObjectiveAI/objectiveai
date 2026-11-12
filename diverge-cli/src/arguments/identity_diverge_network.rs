//! What `diverge identity.diverge.network` takes.

use clap::Subcommand;

/// `identity.diverge.network`, Diverge's own OpenID Connect provider,
/// worked on from this host. Every command but `accounts list` and
/// `accounts login` names an account by its `<ISSUER>` and its `<ID>`,
/// as `accounts list` prints them. An account's issuer is who issued
/// its own sign-in — `github`, `google` — and never
/// `identity.diverge.network` itself.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum IdentityDivergeNetwork {
    /// The host's accounts there: each a user of an issuer — GitHub,
    /// Google — which owns every subject that begins with its issuer
    /// and its id.
    #[command(subcommand)]
    Accounts(Accounts),
    /// The devices' logins that wait to be decided.
    #[command(subcommand)]
    Logins(Logins),
    /// An account's subjects.
    #[command(subcommand)]
    Subjects(Subjects),
    /// A subject's refresh tokens: where it is signed in.
    #[command(name = "refresh-tokens", subcommand)]
    RefreshTokens(RefreshTokens),
}

/// The host's accounts at `identity.diverge.network`, under the root.
/// No command prints a token.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Accounts {
    /// Every account on the host, as `{"account","issuer","id"}`, one
    /// JSON line each. Nothing is asked of anybody.
    List,
    /// What an account's issuer says of its user, now: one JSON line,
    /// `{"account","issuer","id","info"}`, the `info` the issuer's
    /// own object — a GitHub user, or Google's OpenID Connect claims
    /// — as the issuer wrote it. Asked of the issuer every time, and
    /// kept nowhere.
    Info {
        /// The account's issuer, by its name.
        issuer: String,
        /// The account's id at that issuer.
        id: String,
    },
    /// Sign in to an account, at its issuer, by a device code: the
    /// code printed, the issuer's page opened in this host's browser,
    /// and the command ended there. The wait for the code's approval
    /// goes on apart from it, and the account is written once the
    /// issuer grants it — replacing one already there — which
    /// `accounts list` then shows.
    Login {
        /// The issuer to sign in at, by its name: `github`, `google`.
        /// A name that is none is refused with the ones there are.
        issuer: String,
        /// Open no browser: the line printed names the page, for a
        /// person to open.
        #[arg(long)]
        no_browser: bool,
    },
}

/// The devices' logins at `identity.diverge.network`.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Logins {
    /// A login that waits, by its user code: one JSON line,
    /// `{"user_code","ip","user_agent","scope","created","expires"}`
    /// — where the device asked from, and when the login expires.
    Show {
        /// The asking account's issuer.
        issuer: String,
        /// The asking account's id.
        id: String,
        /// The login's user code, as the device printed it.
        user_code: String,
    },
}

/// An account's subjects at `identity.diverge.network`.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Subjects {
    /// Every subject of the account's that is signed in somewhere, as
    /// `{"subject","name","refresh_tokens"}`, one JSON line each.
    List {
        /// The account's issuer.
        issuer: String,
        /// The account's id.
        id: String,
    },
}

/// A subject's refresh tokens at `identity.diverge.network`: one for
/// each place it is signed in. No command prints one.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum RefreshTokens {
    /// Every refresh token of the account's subject of that name, the
    /// oldest first, as
    /// `{"id","created","ip","user_agent","last_used","last_ip"}`,
    /// one JSON line each.
    List {
        /// The account's issuer.
        issuer: String,
        /// The account's id.
        id: String,
        /// The subject's name: its third part.
        name: String,
    },
    /// End one of them: what it signed in is signed out once the
    /// tokens it holds expire. Printed as `{"ended":…}`.
    End {
        /// The account's issuer.
        issuer: String,
        /// The account's id.
        id: String,
        /// The subject's name: its third part.
        name: String,
        /// The refresh token's id, as `refresh-tokens list` prints it.
        refresh_token: String,
    },
}
