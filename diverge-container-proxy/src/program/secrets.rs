//! The secrets, fed to the program's server and collected from it:
//! `POST /secrets` at every activation, `POST /secrets/collect` at
//! every inactive.

use std::fmt;

use diverge_sdk::container_proxy::inside::secrets::{collect, feed};
use diverge_sdk::shared::containers::secrets::Values;
use diverge_sdk::shared::error::Error;
use reqwest::header::CONTENT_TYPE;

use super::{Upstream, refused, status_error};

/// Feed `values` to the program's server: `POST /secrets`, held until
/// the server has applied them. A `2xx` is the values applied, its
/// body unread; a non-`2xx` is the server refusing them, in its own
/// words, and a server that cannot be reached is the refusal too.
/// Nothing of the values is in any error this makes: a refusal says
/// what the server itself chose to say, and no more.
pub async fn feed(upstream: &Upstream, values: Values) -> Result<(), Error> {
    let body = serde_json::to_vec(&feed::request::Request(values))
        .map_err(|error| secrets(format!("the values did not serialize: {}", unquoted(&error))))?;
    let response = upstream
        .http()
        .post(upstream.url("/secrets"))
        .header(CONTENT_TYPE, "application/json")
        .body(body)
        .send()
        .await;
    match response {
        Err(error) => Err(refused(&error)),
        Ok(response) if !response.status().is_success() => Err(status_error(response).await),
        Ok(_) => Ok(()),
    }
}

/// Collect the rotating secrets from the program's server:
/// `POST /secrets/collect` with their `names`, answered with a value
/// for every one of them. A `2xx` whose body holds every name is the
/// values; a non-`2xx` is the server refusing, in its own words, and
/// a server that cannot be reached is the refusal too. A body that is
/// not the response, or one with a name missing, is the error of kind
/// `secrets` — naming the name, never a value — since a rotating
/// secret the program does not give back is a slot the daemon cannot
/// write.
pub async fn collect(upstream: &Upstream, names: &[String]) -> Result<Values, Error> {
    let body = serde_json::to_vec(&collect::request::Request { names: names.to_vec() })
        .map_err(|error| secrets(format!("the names did not serialize: {}", unquoted(&error))))?;
    let response = upstream
        .http()
        .post(upstream.url("/secrets/collect"))
        .header(CONTENT_TYPE, "application/json")
        .body(body)
        .send()
        .await;
    let response = match response {
        Err(error) => return Err(refused(&error)),
        Ok(response) if !response.status().is_success() => return Err(status_error(response).await),
        Ok(response) => response,
    };
    let body = response.bytes().await.map_err(|error| refused(&error))?;
    let collect::response::Response(values) = serde_json::from_slice(&body)
        .map_err(|error| secrets(format!("the collect answer did not parse: {}", unquoted(&error))))?;
    for name in names {
        if !values.contains_key(name) {
            return Err(secrets(format!("the program did not give back the secret `{name}`")));
        }
    }
    Ok(values)
}

/// A failure of the secrets' own, in the wire's one error shape:
/// kind `secrets`.
pub fn secrets(error: impl fmt::Display) -> Error {
    Error(serde_json::json!({
        "kind": "secrets",
        "error": error.to_string(),
    }))
}

/// A JSON error without its text: serde quotes the unexpected datum
/// in its message — `invalid type: string "…"` — and a datum in a
/// secrets body may be a value, so only where it went wrong is said.
fn unquoted(error: &serde_json::Error) -> String {
    let category = match error.classify() {
        serde_json::error::Category::Io => "an io error",
        serde_json::error::Category::Syntax => "a syntax error",
        serde_json::error::Category::Data => "a data error",
        serde_json::error::Category::Eof => "an unexpected end",
    };
    format!("{category} at line {} column {}", error.line(), error.column())
}
