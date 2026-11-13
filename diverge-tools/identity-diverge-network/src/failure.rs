//! Why a tool call did not do what it was asked.

use std::fmt;

/// A tool call's failure, said to the caller as the tool's error
/// result: the request was the tool's to serve, and the caller should
/// read why it was not done.
#[derive(Debug)]
pub enum Failure {
    /// The tool has not been registered yet, so it does not know which
    /// template's accounts it keeps.
    Unregistered,
    /// The tool's parameters are not what it takes.
    Parameters(String),
    /// The issuer is none there is: what was given, and the ones
    /// there are.
    Issuer(String),
    /// The page's request is no person's.
    Person(String),
    /// The tool holds no such account.
    NoAccount(String),
    /// The account's tokens are no longer good at its issuer.
    SignedOut(String),
    /// The daemon grants the tool no login.
    Forbidden,
    /// Another login of the identity, or a reader of it, holds its
    /// lock on the daemon's host.
    Held,
    /// The daemon's login ended denied or expired, which a login the
    /// tool approved itself does not.
    Unapproved(&'static str),
    /// The daemon's login was approved as another subject.
    OtherSubject(String),
    /// The login ended with no outcome.
    Ended,
    /// The daemon could not be reached, or answered with an error.
    Daemon(String),
    /// The database could not be reached, or refused a statement.
    Store(String),
    /// An account's issuer, or `identity.diverge.network`, could not
    /// be asked, or refused.
    Identity(String),
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Failure::Unregistered => f.write_str("the tool has not been registered yet"),
            Failure::Parameters(why) => write!(f, "the parameters are not what this tool takes: {why}"),
            Failure::Issuer(why) => write!(f, "{why}; an account's issuer is who issued its own sign-in, never identity.diverge.network"),
            Failure::Person(why) => write!(f, "the page is a person's, and this request is not one's: {why}"),
            Failure::NoAccount(account) => write!(f, "the tool holds no account `{account}`: `list_accounts` lists the ones it does. A person adds an account at the tool's page"),
            Failure::SignedOut(account) => write!(f, "the account `{account}` is signed out at its issuer: a person signs it in again at the tool's page"),
            Failure::Forbidden => f.write_str("the daemon grants this tool no login: it needs `{\"identities\": [\"login\"]}` for a subject not on the host yet, and `{\"identities\": {\"actions\": [\"login\"], \"within\": \"any\"}}` for one that is; nothing was signed in"),
            Failure::Held => f.write_str("another login of that identity, or a reader of it, holds it on the daemon's host right now; nothing was signed in, try again"),
            Failure::Unapproved(how) => write!(f, "the daemon's login ended {how} though the tool approved it; nothing was signed in"),
            Failure::OtherSubject(subject) => write!(f, "the login was approved as `{subject}`, another subject; nothing was signed in"),
            Failure::Ended => f.write_str("the daemon's login ended with no outcome; nothing is known to be signed in"),
            Failure::Daemon(why) => write!(f, "the daemon could not be asked: {why}; nothing was signed in"),
            Failure::Store(why) => write!(f, "the accounts could not be read or written: {why}"),
            Failure::Identity(why) => write!(f, "{why}"),
        }
    }
}

impl std::error::Error for Failure {}

impl From<sqlx::Error> for Failure {
    fn from(error: sqlx::Error) -> Self {
        Failure::Store(error.to_string())
    }
}
