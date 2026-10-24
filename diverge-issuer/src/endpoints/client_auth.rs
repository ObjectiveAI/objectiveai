//! Who the client is.

use axum::http::{HeaderMap, header};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

use super::OAuthError;
use crate::serve::Issuer;
use crate::store::clients::{Client, Kind};
use crate::tokens;

/// The client a request at the token, device authorization or
/// revocation endpoint is from. The rules, as RFC 6749 §2.3 states
/// them:
///
/// 1. A confidential client presents its id and its secret, either as
///    `Basic` credentials — each form-urlencoded, then joined by `:`,
///    then base64 — or as the form's `client_id` and `client_secret`;
///    never both, which is `invalid_request`.
/// 2. A public client presents its id alone, as the form's
///    `client_id`; a secret beside it is refused.
/// 3. With `Basic`, a form `client_id` that names another client is
///    `invalid_request`.
/// 4. A client there is no such one of, a confidential client whose
///    secret is absent or wrong, and a public client that presents a
///    secret are each `invalid_client`, with `401 Unauthorized`.
pub async fn authenticate(
    issuer: &Issuer,
    headers: &HeaderMap,
    client_id: Option<&str>,
    client_secret: Option<&str>,
) -> Result<Client, OAuthError> {
    let basic = basic(headers)?;
    let used = basic.is_some();
    let (id, secret) = match basic {
        Some((id, secret)) => {
            if client_secret.is_some() {
                return Err(OAuthError::invalid_request("the client authenticated twice"));
            }
            if client_id.is_some_and(|given| given != id) {
                return Err(OAuthError::invalid_request("`client_id` is not the client of the credentials"));
            }
            (id, Some(secret))
        }
        None => match client_id {
            Some(id) => (id.to_string(), client_secret.map(str::to_string)),
            None => return Err(OAuthError::invalid_client(false)),
        },
    };
    let client = issuer.store.clients().load(&id).await?.ok_or(OAuthError::invalid_client(used))?;
    let good = match (client.kind, &client.secret_hash, &secret) {
        (Kind::Confidential, Some(kept), Some(secret)) => equal(kept.as_bytes(), tokens::hash(secret).as_bytes()),
        (Kind::Public, None, None) => true,
        _ => false,
    };
    if !good {
        return Err(OAuthError::invalid_client(used));
    }
    Ok(client)
}

/// The id and the secret of an `Authorization: Basic` header, each
/// form-urlencoding undone; `None` when there is no such header. A
/// `Basic` header that is not base64 of `id:secret` is
/// `invalid_request`.
fn basic(headers: &HeaderMap) -> Result<Option<(String, String)>, OAuthError> {
    let Some(value) = headers.get(header::AUTHORIZATION) else {
        return Ok(None);
    };
    let malformed = || OAuthError::invalid_request("the `Basic` credentials are malformed");
    let value = value.to_str().map_err(|_| malformed())?;
    let Some((scheme, encoded)) = value.split_once(' ') else {
        return Ok(None);
    };
    if !scheme.eq_ignore_ascii_case("basic") {
        return Ok(None);
    }
    let decoded = STANDARD.decode(encoded.trim()).map_err(|_| malformed())?;
    let decoded = String::from_utf8(decoded).map_err(|_| malformed())?;
    let (id, secret) = decoded.split_once(':').ok_or_else(malformed)?;
    Ok(Some((unescape(id), unescape(secret))))
}

/// `text` with its form-urlencoding undone: `+` a space, `%XX` a byte.
/// It is parsed as a form's lone key, which is exactly that.
fn unescape(text: &str) -> String {
    url::form_urlencoded::parse(text.as_bytes()).next().map(|(key, _)| key.into_owned()).unwrap_or_default()
}

/// Whether `a` and `b` are equal, in time that depends on their length
/// alone.
fn equal(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}
