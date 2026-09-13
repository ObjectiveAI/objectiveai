//! The agent, told to the agent's server once: `POST /register`.

use diverge_container_proxy_sdk::agent::register::request::Request;
use diverge_provider_sdk::shared::error::Error;
use reqwest::header::CONTENT_TYPE;
use serde_json::Value;

use super::{Upstream, refused, status_error};

/// Register `agent` with the agent's server. A `2xx` is the agent
/// held for the container's life; a non-`2xx` is the image refusing
/// it, in its own words, and a server that cannot be reached is the
/// refusal too.
pub async fn register(upstream: &Upstream, agent: Value) -> Result<(), Error> {
    let body = serde_json::to_vec(&Request { agent }).map_err(|error| {
        Error(serde_json::json!({
            "kind": "agent",
            "error": format!("the agent did not serialize: {error}"),
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
