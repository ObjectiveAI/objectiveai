//! The arguments, told to the program's server once: `POST
//! /register`.

use diverge_container_proxy_sdk::register::request::Request;
use diverge_provider_sdk::shared::error::Error;
use reqwest::header::CONTENT_TYPE;
use serde_json::Value;

use super::{Upstream, refused, status_error};

/// Register `arguments` with the program's server. A `2xx` is the
/// arguments held for the container's life; a non-`2xx` is the image
/// refusing them, in its own words, and a server that cannot be
/// reached is the refusal too.
pub async fn register(upstream: &Upstream, arguments: Value) -> Result<(), Error> {
    let body = serde_json::to_vec(&Request { arguments }).map_err(|error| {
        Error(serde_json::json!({
            "kind": "program",
            "error": format!("the arguments did not serialize: {error}"),
        }))
    })?;
    let response = upstream
        .http()
        .post(upstream.url("/register"))
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
