//! Who a login signed in.

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use reqwest::header::ACCEPT;
use serde_json::Value;

use super::account::usable;
use super::pending::Tokens;
use super::{Error, Who};

/// The id of the user `tokens` are for, as `who` learns it — see the
/// [rules](super), rule 3. The rules:
///
/// 1. [`Who::Asked`]: the URL is asked with the access token as its
///    bearer, and the answer's `id` — a number, written as its digits,
///    or text — is the user's. A provider that answered no access
///    token, an answer that is not a success, and one with no `id`
///    are [`Error::Id`].
/// 2. [`Who::IdToken`]: the ID token's payload is read, its signature
///    not checked — OpenID Connect Core §3.1.3.7 allows it for an ID
///    token taken from the token endpoint itself, over TLS — and its
///    `sub` is the user's. No ID token, and one with no `sub`, are
///    [`Error::Id`].
/// 3. An id that cannot name a file is [`Error::Id`].
pub(super) async fn who(http: &reqwest::Client, who: Who, tokens: &Tokens) -> Result<String, Error> {
    let id = match who {
        // Rule 1.
        Who::Asked(url) => {
            let token = tokens.access_token.as_deref().ok_or(Error::Id)?;
            let response = http.get(url).bearer_auth(token).header(ACCEPT, "application/json").send().await.map_err(Error::Http)?;
            if !response.status().is_success() {
                return Err(Error::Id);
            }
            let answer: Value = response.json().await.map_err(Error::Http)?;
            match answer.get("id") {
                Some(Value::Number(id)) => id.to_string(),
                Some(Value::String(id)) => id.clone(),
                _ => return Err(Error::Id),
            }
        }
        // Rule 2.
        Who::IdToken => {
            let payload = tokens.id_token.as_deref().and_then(|token| token.split('.').nth(1)).ok_or(Error::Id)?;
            let claims: Value = URL_SAFE_NO_PAD.decode(payload).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok()).ok_or(Error::Id)?;
            claims.get("sub").and_then(Value::as_str).ok_or(Error::Id)?.to_string()
        }
    };
    // Rule 3.
    if !usable(&id) {
        return Err(Error::Id);
    }
    Ok(id)
}
