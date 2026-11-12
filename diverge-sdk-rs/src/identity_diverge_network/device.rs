//! A request to a provider's device or token endpoint, and its answer.

use reqwest::header::ACCEPT;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::Error;

/// What an endpoint answered: `T`, or a refusal.
pub(super) enum Answer<T> {
    /// The answer asked for.
    Answered(T),
    /// The refusal: its error code, and what the provider said of it.
    Refused {
        /// The error code.
        code: String,
        /// The description, where there is one.
        description: Option<String>,
    },
}

/// POST `form` to `url`, form-encoded, asking for JSON, and read the
/// answer. The rules:
///
/// 1. **JSON is asked for.** GitHub answers a form unless it is.
/// 2. **A refusal is a body with an `error`**, whatever the status:
///    RFC 6749 §5.2 has a refusal answered `400`, and GitHub answers
///    one `200`, so the status says nothing and the body is read.
/// 3. Any other body with a success status is `T`; one that is not
///    `T`, and any other body with another status, is
///    [`Error::Answer`], with the status and the body.
pub(super) async fn post<T: DeserializeOwned>(http: &reqwest::Client, url: &str, form: &[(&str, &str)]) -> Result<Answer<T>, Error> {
    let response = http.post(url).header(ACCEPT, "application/json").form(form).send().await.map_err(Error::Http)?;
    let status = response.status();
    let body = response.bytes().await.map_err(Error::Http)?;
    let unread = || Error::Answer { url: url.to_string(), status: status.as_u16(), body: String::from_utf8_lossy(&body).into_owned() };
    let value: Value = serde_json::from_slice(&body).map_err(|_| unread())?;
    // Rule 2.
    if let Some(code) = value.get("error").and_then(Value::as_str) {
        let description = value.get("error_description").and_then(Value::as_str).map(str::to_string);
        return Ok(Answer::Refused { code: code.to_string(), description });
    }
    // Rule 3.
    if !status.is_success() {
        return Err(unread());
    }
    serde_json::from_value(value).map(Answer::Answered).map_err(|_| unread())
}
