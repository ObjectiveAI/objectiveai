//! A request to an issuer's token or device endpoint, and its answer.

use serde::Deserialize;
use serde::de::DeserializeOwned;

use super::Error;

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

/// POST `form` to `url`, form-encoded, and read the answer: a success
/// status as `T`, any other as a [`Refusal`]. A body that is neither
/// is [`Error::Answer`], with the status and the body.
pub(super) async fn post<T: DeserializeOwned>(http: &reqwest::Client, url: &str, form: &[(&str, &str)]) -> Result<Answer<T>, Error> {
    let response = http.post(url).form(form).send().await.map_err(Error::Http)?;
    let status = response.status();
    let body = response.bytes().await.map_err(Error::Http)?;
    let unread = || Error::Answer { url: url.to_string(), status: status.as_u16(), body: String::from_utf8_lossy(&body).into_owned() };
    if status.is_success() {
        return serde_json::from_slice(&body).map(Answer::Answered).map_err(|_| unread());
    }
    serde_json::from_slice(&body).map(Answer::Refused).map_err(|_| unread())
}
