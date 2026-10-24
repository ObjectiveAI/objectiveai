//! What `diverge issuer` takes.

use clap::{ArgGroup, Subcommand, ValueEnum};

/// The issuer on this host: started, stopped, and the identities and
/// clients in its `issuer.sqlite` worked on. The identities and the
/// clients are written to the database directly, whether the issuer
/// runs or not; the issuer reads both at every request.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Issuer {
    /// Start the issuer on the root, detached, its output in
    /// `<root>/issuer.log`; the command ends once the issuer holds the
    /// root's lock.
    Start,
    /// Stop the issuer running on the root, asked over its own port on
    /// this host; the command ends once the root's lock is free.
    Stop,
    /// The identities a person signs in as.
    #[command(subcommand)]
    Identities(Identities),
    /// The clients that ask for tokens.
    #[command(subcommand)]
    Clients(Clients),
}

/// What is done to the identities.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Identities {
    /// Add an identity, its password read from stdin to its end, the
    /// trailing line end dropped. A subject already held is refused.
    Add {
        /// The subject: the `sub` of every token issued to it.
        subject: String,
    },
    /// Every identity, by subject, with when it was added: one JSON
    /// line each, never a password's hash.
    List,
    /// Take an identity out, and every code and refresh token issued
    /// to it.
    Remove {
        /// The subject.
        subject: String,
    },
    /// Change an identity's password, read from stdin to its end, and
    /// revoke every refresh token issued to it.
    Password {
        /// The subject.
        subject: String,
    },
}

/// What is done to the clients.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Clients {
    /// Add a client. A confidential one's secret is minted into the
    /// root's keyring as `issuer/clients/<id>`, and only its hash is
    /// kept by the issuer; no command prints it.
    #[command(group(ArgGroup::new("kind").required(true).args(["public", "confidential"])))]
    Add {
        /// The client's id.
        id: String,
        /// A client with no secret: the CLI, the viewer, a page.
        #[arg(long)]
        public: bool,
        /// A client with a secret.
        #[arg(long)]
        confidential: bool,
        /// A grant it may use; given once per grant.
        #[arg(long = "grant", value_enum, required = true)]
        grants: Vec<Grant>,
        /// An audience its access tokens may be for; given once per
        /// audience, the first the one a request naming none gets.
        #[arg(long = "audience", required = true)]
        audiences: Vec<String>,
        /// A redirect URI the authorization code may be sent to; given
        /// once per URI, and exactly when it has `authorization_code`.
        #[arg(long = "redirect")]
        redirects: Vec<String>,
    },
    /// Every client, whole, by id: one JSON line each, never a
    /// secret's hash.
    List,
    /// Take a client out, every code and refresh token issued to it,
    /// and its secret's keyring entry where it has one.
    Remove {
        /// The client's id.
        id: String,
    },
}

/// A grant, as the issuer names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Grant {
    /// A person signs in on the issuer's page; the client is sent a
    /// code.
    #[value(name = "authorization_code")]
    AuthorizationCode,
    /// A person signs in with a code the client shows.
    #[value(name = "device_code")]
    DeviceCode,
    /// A refresh token, spent for new tokens.
    #[value(name = "refresh_token")]
    RefreshToken,
    /// The client's own secret, for a token naming the client.
    #[value(name = "client_credentials")]
    ClientCredentials,
}
