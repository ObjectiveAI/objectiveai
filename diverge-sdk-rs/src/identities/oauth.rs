//! A request to an issuer's token or device endpoint, and its answer.

use serde::Deserialize;
use serde::de::DeserializeOwned;

use super::proof::{HEADER, NONCE, proof};
use super::{Error, Key};

/// A token endpoint's answer, RFC 6749 §5.1.
#[derive(Debug, Clone, Deserialize)]
pub(super) struct Tokens {
    /// The access token.
    pub(super) access_token: String,
    /// The seconds it is good for.
    pub(super) expires_in: Option<u64>,
    /// A refresh token, new or rotated.
    pub(super) refresh_token: Option<String>,
    /// The ID token.
    pub(super) id_token: Option<String>,
    /// The scope granted, where it differs from the one asked.
    pub(super) scope: Option<String>,
}

/// An endpoint's refusal, RFC 6749 §5.2.
#[derive(Debug, Clone, Deserialize)]
pub(super) struct Refusal {
    /// The error code.
    pub(super) error: String,
    /// What the issuer says went wrong.
    pub(super) error_description: Option<String>,
}

/// What an endpoint answered: `T` with a success status, or a refusal
/// with another.
pub(super) enum Answer<T> {
    /// The answer asked for.
    Answered(T),
    /// The refusal.
    Refused(Refusal),
}

/// The refusal an issuer answers a proof that lacks the nonce it
/// wants, RFC 9449 §8.
const USE_NONCE: &str = "use_dpop_nonce";

/// POST `form` to `url`, form-encoded, and read the answer: a success
/// status as `T`, any other as a [`Refusal`]. A body that is neither
/// is [`Error::Answer`], with the status and the body. The rules:
///
/// 1. **With a key, a proof.** Where `key` is given the request
///    carries a `DPoP` header: a [`proof`] for this request, signed
///    with the key. A request to the device authorization endpoint
///    proves nothing, and is given no key.
/// 2. **A nonce asked for is given, once.** An issuer that refuses the
///    request as `use_dpop_nonce` and names a nonce in its `DPoP-Nonce`
///    header is asked again, once, with a new proof carrying that
///    nonce. Its second answer is the answer, whatever it is.
pub(super) async fn post<T: DeserializeOwned>(http: &reqwest::Client, url: &str, form: &[(&str, &str)], key: Option<&Key>) -> Result<Answer<T>, Error> {
    let mut nonce: Option<String> = None;
    loop {
        let mut request = http.post(url).form(form);
        // Rule 1.
        if let Some(key) = key {
            request = request.header(HEADER, proof(key, url, nonce.as_deref()));
        }
        let response = request.send().await.map_err(Error::Http)?;
        let status = response.status();
        let wanted = response.headers().get(NONCE).and_then(|value| value.to_str().ok()).map(str::to_string);
        let body = response.bytes().await.map_err(Error::Http)?;
        let unread = || Error::Answer { url: url.to_string(), status: status.as_u16(), body: String::from_utf8_lossy(&body).into_owned() };
        if status.is_success() {
            return serde_json::from_slice(&body).map(Answer::Answered).map_err(|_| unread());
        }
        let refusal: Refusal = serde_json::from_slice(&body).map_err(|_| unread())?;
        // Rule 2.
        if key.is_some() && nonce.is_none() && refusal.error == USE_NONCE && wanted.is_some() {
            nonce = wanted;
            continue;
        }
        return Ok(Answer::Refused(refusal));
    }
}
