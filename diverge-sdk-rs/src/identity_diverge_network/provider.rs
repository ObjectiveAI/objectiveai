//! The external providers.

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

/// An external provider, as `identity.diverge.network` is a client of
/// it: everything a login needs of it, and nothing else — see the
/// [rules](super), rule 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Provider {
    /// The provider's name: the word it is asked for by, and the first
    /// part of every subject of an account of its.
    pub name: &'static str,
    /// The client `identity.diverge.network` is at the provider: the
    /// one its tokens must be issued to.
    pub client_id: &'static str,
    /// That client's secret, where the provider asks for one to spend
    /// a device code. One that is here is one the provider itself does
    /// not hold to be a secret: it is in every copy of this crate.
    pub client_secret: Option<&'static str>,
    /// Where a device code is asked for, RFC 8628 §3.1.
    pub device_endpoint: &'static str,
    /// Where it is spent, RFC 8628 §3.4.
    pub token_endpoint: &'static str,
    /// The scope asked for: the least that has the provider say who
    /// the user is.
    pub scope: &'static str,
    /// How the user is learned.
    pub who: Who,
}

/// Every provider there is. One more is one more entry here.
pub const PROVIDERS: [Provider; 2] = [
    // GitHub: an OAuth app, the device flow switched on. No scope:
    // who the user is needs none. Its token names nobody, so GitHub
    // is asked.
    Provider {
        name: "github",
        client_id: "Ov23lij4pJSNsCRscWua",
        client_secret: None,
        device_endpoint: "https://github.com/login/device/code",
        token_endpoint: "https://github.com/login/oauth/access_token",
        scope: "",
        who: Who::Asked("https://api.github.com/user"),
    },
    // Google: a client of the kind for TVs and limited-input devices,
    // the one kind with a device flow — which asks for the client's
    // secret, and which Google does not hold to be one. `openid` has
    // the token endpoint answer an ID token, which names the user and
    // the client it was issued to.
    Provider {
        name: "google",
        client_id: "562291546590-t128jl54jlfqo70kj9otoqspnnsjqo69.apps.googleusercontent.com",
        client_secret: Some("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"),
        device_endpoint: "https://oauth2.googleapis.com/device/code",
        token_endpoint: "https://oauth2.googleapis.com/token",
        scope: "openid",
        who: Who::IdToken,
    },
];

/// The provider named `name`, exactly, or [`Error::Provider`] with
/// the names there are.
pub fn provider(name: &str) -> Result<&'static Provider, Error> {
    PROVIDERS.iter().find(|provider| provider.name == name).ok_or_else(|| Error::Provider { named: name.to_string(), known: PROVIDERS.iter().map(|provider| provider.name).collect() })
}
