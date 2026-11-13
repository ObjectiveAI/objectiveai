//! An account's tokens, and keeping them good.

use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::device::{Answer, post};
use super::pending::Answered;
use super::who::claims;
use super::{Error, Issuer, Which};

/// The seconds before its expiry a token is renewed at: one that
/// would end while the request it is for is on its way is not used.
pub const EARLY: i64 = 60;

/// What an account's issuer answered its login, and every renewal
/// since, with: the account itself, wherever it is kept. This module
/// keeps one in a file — see [`Accounts`](super::Accounts); a program
/// with a store of its own keeps one there, and has it kept good by
/// the same two functions, [`good`](Self::good) and
/// [`renew`](Self::renew), under whatever lock its store has.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tokens {
    /// The access token.
    pub access_token: Option<String>,
    /// When the access token expires, in seconds since the Unix epoch,
    /// where the issuer said; one that does not expire has none.
    pub access_expires: Option<i64>,
    /// The refresh token, where the issuer answered one.
    pub refresh_token: Option<String>,
    /// The ID token, where the issuer answered one.
    pub id_token: Option<String>,
}

/// Tokens are printed as which of them there are, and never as what
/// they are.
impl fmt::Debug for Tokens {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tokens")
            .field("access_token", &self.access_token.is_some())
            .field("access_expires", &self.access_expires)
            .field("refresh_token", &self.refresh_token.is_some())
            .field("id_token", &self.id_token.is_some())
            .finish()
    }
}

impl Tokens {
    /// The token, `which` of the two, where there is one good for
    /// longer than [`EARLY`] from `now`. The rules:
    ///
    /// 1. **The access token** expires when the issuer said it would;
    ///    one the issuer gave no expiry — GitHub's — does not expire.
    /// 2. **The ID token** expires at its own `exp`, read from it; one
    ///    with none is not good.
    pub fn good(&self, which: Which, now: i64) -> Option<&str> {
        let bound = now.saturating_add(EARLY);
        match which {
            Which::Access => self.access_token.as_deref().filter(|_| self.access_expires.is_none_or(|expires| expires > bound)),
            Which::Id => {
                let token = self.id_token.as_deref()?;
                let expires = claims(token)?.get("exp").and_then(Value::as_i64)?;
                (expires > bound).then_some(token)
            }
        }
    }

    /// Renew the tokens at `issuer`, at `now`: the refresh token spent
    /// at the issuer's token endpoint, RFC 6749 §6, as the client
    /// `identity.diverge.network` is there, and what was answered
    /// taken in — the access token and its expiry, and the ID token
    /// and the refresh token where the issuer answered new ones.
    ///
    /// Tokens with no refresh token, and an answer with no access
    /// token, are a refusal — [`Error::Refused`] — as the issuer's own
    /// is: the account is signed out there, and signs in again.
    /// Nothing is written here: the caller holds whatever lock its
    /// store has, and keeps what this leaves.
    pub async fn renew(&mut self, http: &reqwest::Client, issuer: &Issuer, now: i64) -> Result<(), Error> {
        let refused = |code: &str| Error::Refused { code: code.to_string(), description: None };
        let refresh_token = self.refresh_token.clone().ok_or_else(|| refused("no_refresh_token"))?;
        let mut form = vec![("grant_type", "refresh_token"), ("refresh_token", refresh_token.as_str()), ("client_id", issuer.client_id)];
        if let Some(secret) = issuer.client_secret {
            form.push(("client_secret", secret));
        }
        let answered = match post::<Answered>(http, issuer.token_endpoint, &form).await? {
            Answer::Answered(answered) => answered,
            Answer::Refused { code, description } => return Err(Error::Refused { code, description }),
        };
        self.access_token = Some(answered.access_token.ok_or_else(|| refused("no_access_token"))?);
        self.access_expires = answered.expires_in.map(|seconds| now.saturating_add(i64::try_from(seconds).unwrap_or(i64::MAX)));
        if let Some(id_token) = answered.id_token {
            self.id_token = Some(id_token);
        }
        if let Some(refresh_token) = answered.refresh_token {
            self.refresh_token = Some(refresh_token);
        }
        Ok(())
    }
}
