//! The arguments, told to the program's server once: `POST
//! /register`, answered with the dependencies the program declares.

use diverge_sdk::container_proxy::inside::register::request::Request;
use diverge_sdk::container_proxy::inside::register::response::Response;
use diverge_sdk::shared::containers::dependencies::Template;
use diverge_sdk::shared::error::Error;
use reqwest::header::CONTENT_TYPE;
use serde_json::Value;

use super::{Upstream, refused, status_error};

/// Register `arguments` with the program's server. A `2xx` is the
/// arguments held for the container's life, and its body the
/// dependencies the program declares, which an agent container's `Begun` carries
/// and a tool container's begin ignores; a non-`2xx` is the
/// image refusing them, in its own words, and a server that cannot be
/// reached is the refusal too. So is a `2xx` whose body is not the
/// response: the dependencies are part of registration, and a
/// registration the proxy cannot read has not happened.
pub async fn register(upstream: &Upstream, arguments: Value) -> Result<Vec<Template>, Error> {
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
    let response = match response {
        Err(error) => return Err(refused(&error)),
        Ok(response) if !response.status().is_success() => return Err(status_error(response).await),
        Ok(response) => response,
    };
    let body = response.bytes().await.map_err(|error| refused(&error))?;
    serde_json::from_slice::<Response>(&body)
        .map(|response| response.dependencies)
        .map_err(|error| {
            Error(serde_json::json!({
                "kind": "program",
                "error": format!("the register answer did not parse: {error}"),
            }))
        })
}
