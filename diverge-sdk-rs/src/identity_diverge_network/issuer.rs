//! The issuers.

use super::Error;

/// How the user a login signed in is learned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Who {
    /// Asked: a `GET` of this URL with the access token as its bearer
    /// answers a JSON object whose `id` — a number, or text — is the
    /// user's.
    Asked(&'static str),
    /// Read: the `sub` of the ID token the token endpoint answered.
    IdToken,
}

/// Which of an account's tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Which {
    /// The access token.
    Access,
    /// The ID token.
    Id,
}

/// An issuer of accounts' own sign-ins — GitHub, Google — as
/// `identity.diverge.network` is a client of it: everything this
/// module needs of it, and nothing else — see the [rules](super),
/// rule 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Issuer {
    /// The issuer's name: the word it is asked for by, and the first
    /// part of every subject of an account of its.
    pub name: &'static str,
    /// The client `identity.diverge.network` is at the issuer: the one
    /// an account's tokens must be issued to.
    pub client_id: &'static str,
    /// That client's secret, where the issuer asks for one to spend a
    /// device code. One that is here is one the issuer itself does not
    /// hold to be a secret: it is in every copy of this crate.
    pub client_secret: Option<&'static str>,
    /// Where a device code is asked for, RFC 8628 §3.1.
    pub device_endpoint: &'static str,
    /// Where it is spent, RFC 8628 §3.4, and a refresh token after it.
    pub token_endpoint: &'static str,
    /// The scope asked for: what has the issuer say who the user is,
    /// and what [`info`](super::Accounts::info) reads of them.
    pub scope: &'static str,
    /// How the user is learned.
    pub who: Who,
    /// Where the user's own details are read: a `GET` with the access
    /// token as its bearer answers a JSON object, which is the
    /// issuer's own and differs from one issuer to the next.
    pub info_endpoint: &'static str,
    /// Which of the account's tokens `identity.diverge.network` takes
    /// as the account: the one it can tell was issued to its own
    /// client.
    pub presents: Which,
}

/// Every issuer there is. One more is one more entry here.
pub const ISSUERS: [Issuer; 2] = [
    // GitHub: an OAuth app, the device flow switched on. No scope:
    // who the user is, and what GitHub shows of anyone, needs none.
    // Its token names nobody, so GitHub is asked — by this module who
    // the user is, and by identity.diverge.network whether the token
    // is its client's.
    Issuer {
        name: "github",
        client_id: "Ov23lij4pJSNsCRscWua",
        client_secret: None,
        device_endpoint: "https://github.com/login/device/code",
        token_endpoint: "https://github.com/login/oauth/access_token",
        scope: "",
        who: Who::Asked("https://api.github.com/user"),
        info_endpoint: "https://api.github.com/user",
        presents: Which::Access,
    },
    // Google: a client of the kind for TVs and limited-input devices,
    // the one kind with a device flow — which asks for the client's
    // secret, and which Google does not hold to be one. `openid` has
    // the token endpoint answer an ID token, which names the user and
    // the client it was issued to, and so is what is presented;
    // `email` and `profile` are what the user's details are, which
    // Google answers none of without them.
    Issuer {
        name: "google",
        client_id: "562291546590-t128jl54jlfqo70kj9otoqspnnsjqo69.apps.googleusercontent.com",
        client_secret: Some("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"),
        device_endpoint: "https://oauth2.googleapis.com/device/code",
        token_endpoint: "https://oauth2.googleapis.com/token",
        scope: "openid email profile",
        who: Who::IdToken,
        info_endpoint: "https://openidconnect.googleapis.com/v1/userinfo",
        presents: Which::Id,
    },
];

/// The issuer named `name`, exactly, or [`Error::Issuer`] with the
/// names there are.
pub fn issuer(name: &str) -> Result<&'static Issuer, Error> {
    ISSUERS.iter().find(|issuer| issuer.name == name).ok_or_else(|| Error::Issuer { named: name.to_string(), known: ISSUERS.iter().map(|issuer| issuer.name).collect() })
}
